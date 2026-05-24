use std::{
    path::{
        Path,
        PathBuf,
    },
    sync::{
        Arc,
        RwLock,
        atomic::{
            AtomicU64,
            Ordering,
        },
    },
    thread,
};

use nucleo_matcher::{
    Config,
    Matcher,
    Utf32Str,
    pattern::{
        AtomKind,
        CaseMatching,
        Normalization,
        Pattern,
    },
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    provider::{
        Provider,
        SearchResult,
    },
    providers::files::{
        FilesProviderConfig,
        activate_file,
        expand_path,
        file_result_with_subtitle,
    },
};

pub struct FileSearchProvider {
    roots: Vec<RootSpec>,
    index: Arc<RwLock<Vec<IndexedRoot>>>,
    scan_generation: Arc<AtomicU64>,
    min_query_len: usize,
    max_results: usize,
    editor_command: String,
    open_command: String,
}

#[derive(Clone)]
struct RootSpec {
    path: PathBuf,
    alias: String,
}

struct IndexedRoot {
    path: PathBuf,
    alias: String,
    files: Vec<IndexedFile>,
}

struct IndexedFile {
    path: PathBuf,
    relative_path: String,
}

struct MatchedFile<'a> {
    root: &'a IndexedRoot,
    file: &'a IndexedFile,
    score: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FileSearchProviderConfig {
    pub enabled: bool,
    pub roots: Vec<String>,
    pub min_query_len: usize,
    pub max_results: usize,
}

impl Default for FileSearchProviderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            roots: vec![],
            min_query_len: 3,
            max_results: 50,
        }
    }
}

impl FileSearchProvider {
    pub fn new(config: &FileSearchProviderConfig, files_config: &FilesProviderConfig) -> Self {
        let provider = Self {
            roots: prepare_roots(&config.roots),
            index: Arc::new(RwLock::new(vec![])),
            scan_generation: Arc::new(AtomicU64::new(0)),
            min_query_len: config.min_query_len,
            max_results: config.max_results,
            editor_command: files_config.editor_command.clone(),
            open_command: files_config.open_command.clone(),
        };
        provider.start_scan();
        provider
    }

    #[cfg(test)]
    fn wait_for_scan(&self) {
        while self.scan_generation.load(Ordering::SeqCst) != 0 {
            thread::yield_now();
        }
    }
}

impl Provider for FileSearchProvider {
    fn id(&self) -> &'static str {
        "file_search"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim();
        let index = self
            .index
            .read()
            .expect("file search index lock should not be poisoned");
        if query.len() < self.min_query_len || index.is_empty() || self.max_results == 0 {
            return vec![];
        }

        let (roots, query) = scoped_query(&index, query);
        if query.len() < self.min_query_len {
            return vec![];
        }

        let pattern = Pattern::new(
            query,
            CaseMatching::Ignore,
            Normalization::Smart,
            AtomKind::Fuzzy,
        );
        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let mut matches = vec![];

        for root in roots {
            let mut buf = Vec::new();
            matches.extend(root.files.iter().filter_map(|file| {
                pattern
                    .score(Utf32Str::new(&file.relative_path, &mut buf), &mut matcher)
                    .map(|score| MatchedFile { root, file, score })
            }));
        }

        let max_score = matches
            .iter()
            .map(|matched| matched.score)
            .max()
            .unwrap_or(1)
            .max(1);

        matches.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.file.relative_path.cmp(&b.file.relative_path))
                .then_with(|| a.file.path.cmp(&b.file.path))
        });

        matches
            .into_iter()
            .take(self.max_results)
            .map(|matched| {
                let subtitle = indexed_subtitle(&matched.file.path, matched.root);
                file_result_with_subtitle(
                    self.id(),
                    &matched.file.path,
                    false,
                    !self.editor_command.is_empty(),
                    Some(subtitle),
                    matched.score as f32 / max_score as f32,
                )
            })
            .collect()
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        activate_file(id, action, &self.editor_command, &self.open_command)
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        self.start_scan();
        Ok(())
    }
}

impl FileSearchProvider {
    fn start_scan(&self) {
        if self.roots.is_empty() {
            return;
        }

        let roots = self.roots.clone();
        let index = Arc::clone(&self.index);
        let scan_generation = Arc::clone(&self.scan_generation);
        let generation = scan_generation.fetch_add(1, Ordering::SeqCst) + 1;

        thread::spawn(move || {
            let scanned = scan_roots(&roots);
            if scan_generation
                .compare_exchange(generation, 0, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                *index
                    .write()
                    .expect("file search index lock should not be poisoned") = scanned;
            }
        });
    }
}

fn scoped_query<'a>(roots: &'a [IndexedRoot], query: &'a str) -> (Vec<&'a IndexedRoot>, &'a str) {
    let Some((first, rest)) = query.split_once(char::is_whitespace) else {
        return (roots.iter().collect(), query);
    };

    let rest = rest.trim_start();
    if rest.is_empty() {
        return (roots.iter().collect(), query);
    }

    let matches: Vec<&IndexedRoot> = roots
        .iter()
        .filter(|root| root.alias.eq_ignore_ascii_case(first))
        .collect();

    if matches.is_empty() {
        (roots.iter().collect(), query)
    } else {
        (matches, rest)
    }
}

fn prepare_roots(roots: &[String]) -> Vec<RootSpec> {
    let mut roots: Vec<RootSpec> = roots
        .iter()
        .filter_map(|root| build_root_spec(root))
        .collect();
    roots.sort_by(|a, b| a.path.cmp(&b.path));

    let mut deduped: Vec<RootSpec> = vec![];
    for root in roots {
        if deduped
            .iter()
            .any(|existing| root.path.starts_with(&existing.path))
        {
            continue;
        }

        deduped.push(root);
    }

    deduped
}

fn build_root_spec(root: &str) -> Option<RootSpec> {
    let path = match expand_path(root).canonicalize() {
        Ok(path) if path.is_dir() => path,
        Ok(_) => return None,
        Err(err) => {
            eprintln!("skipping file_search root '{root}': {err}");
            return None;
        }
    };

    let alias = path
        .file_name()
        .map(|file_name| file_name.to_string_lossy().to_string())
        .filter(|file_name| !file_name.is_empty())
        .unwrap_or_else(|| path.display().to_string());
    Some(RootSpec { path, alias })
}

fn scan_roots(roots: &[RootSpec]) -> Vec<IndexedRoot> {
    roots.iter().map(scan_root).collect()
}

fn indexed_subtitle(path: &Path, root: &IndexedRoot) -> String {
    let parent = path.parent().unwrap_or(&root.path);
    match parent.strip_prefix(&root.path) {
        Ok(relative) if relative.as_os_str().is_empty() => root.alias.clone(),
        Ok(relative) => format!(
            "{} > {}",
            root.alias,
            relative
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, " > ")
        ),
        Err(_) => format!("{} > {}", root.alias, parent.to_string_lossy()),
    }
}

fn scan_root(root: &RootSpec) -> IndexedRoot {
    let mut files = vec![];
    collect_files(&root.path, &root.path, &mut files);
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    IndexedRoot {
        path: root.path.clone(),
        alias: root.alias.clone(),
        files,
    }
}

fn collect_files(root: &Path, path: &Path, files: &mut Vec<IndexedFile>) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };

        if file_type.is_dir() {
            collect_files(root, &path, files);
        } else if file_type.is_file() {
            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "/");
            files.push(IndexedFile {
                path,
                relative_path,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use super::*;

    fn temp_dir(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("rika-file-search-{test_name}-{nanos}"))
    }

    fn provider(root: &Path) -> FileSearchProvider {
        FileSearchProvider::new(
            &FileSearchProviderConfig {
                enabled: true,
                roots: vec![root.to_string_lossy().to_string()],
                min_query_len: 3,
                max_results: 50,
            },
            &FilesProviderConfig {
                enabled: true,
                editor_command: "nvim".to_string(),
                open_command: "xdg-open".to_string(),
            },
        )
    }

    #[test]
    fn default_config_is_opt_in() {
        let config = FileSearchProviderConfig::default();

        assert!(!config.enabled);
        assert!(config.roots.is_empty());
        assert_eq!(config.min_query_len, 3);
        assert_eq!(config.max_results, 50);
    }

    #[test]
    fn missing_roots_are_skipped() {
        let root = temp_dir("missing");
        let provider = provider(&root);

        assert!(provider.roots.is_empty());
        assert!(provider.search("notes").is_empty());
    }

    #[test]
    fn duplicate_roots_are_indexed_once() {
        let root = temp_dir("duplicate-roots");
        fs::create_dir_all(&root).expect("temp directory should be created");
        fs::write(root.join("notes.md"), "hello").expect("file should be created");
        let equivalent_root = root.join("..").join(
            root.file_name()
                .expect("temp directory should have a file name"),
        );
        let provider = FileSearchProvider::new(
            &FileSearchProviderConfig {
                enabled: true,
                roots: vec![
                    root.to_string_lossy().to_string(),
                    equivalent_root.to_string_lossy().to_string(),
                ],
                min_query_len: 3,
                max_results: 50,
            },
            &FilesProviderConfig {
                enabled: true,
                editor_command: String::new(),
                open_command: "xdg-open".to_string(),
            },
        );
        provider.wait_for_scan();

        assert_eq!(provider.roots.len(), 1);
        assert_eq!(provider.search("notes").len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn child_roots_are_skipped_when_parent_root_is_configured() {
        let root = temp_dir("overlapping-roots");
        let child = root.join("books");
        fs::create_dir_all(&child).expect("child directory should be created");
        fs::write(child.join("notes.md"), "hello").expect("file should be created");
        let provider = FileSearchProvider::new(
            &FileSearchProviderConfig {
                enabled: true,
                roots: vec![
                    root.to_string_lossy().to_string(),
                    child.to_string_lossy().to_string(),
                ],
                min_query_len: 3,
                max_results: 50,
            },
            &FilesProviderConfig {
                enabled: true,
                editor_command: String::new(),
                open_command: "xdg-open".to_string(),
            },
        );
        provider.wait_for_scan();

        assert_eq!(provider.roots.len(), 1);
        assert_eq!(provider.search("notes").len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn short_queries_return_no_results() {
        let root = temp_dir("short-query");
        fs::create_dir_all(&root).expect("temp directory should be created");
        fs::write(root.join("notes.md"), "hello").expect("file should be created");
        let provider = provider(&root);
        provider.wait_for_scan();

        assert!(provider.search("no").is_empty());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn searches_indexed_files_from_configured_roots() {
        let root = temp_dir("results");
        let nested = root.join("docs");
        fs::create_dir_all(&nested).expect("temp directory should be created");
        let file = nested.join("needle_notes.md");
        fs::write(&file, "hello").expect("file should be created");
        let provider = provider(&root);
        provider.wait_for_scan();

        let results = provider.search("needle");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "file_search");
        assert_eq!(results[0].id, file.to_string_lossy());
        assert!(!results[0].id.starts_with("file_search:"));
        assert_eq!(results[0].title, "needle_notes.md");
        assert!(results[0].subtitle.ends_with(" > docs"));
        assert_eq!(results[0].default_action, "open");
        assert!(
            results[0]
                .actions
                .iter()
                .any(|action| action.id == "open_editor")
        );
        assert!(
            results[0]
                .actions
                .iter()
                .any(|action| action.id == "copy_path")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_updates_index_from_background_scan() {
        let root = temp_dir("refresh");
        fs::create_dir_all(&root).expect("temp directory should be created");
        fs::write(root.join("old-notes.md"), "hello").expect("old file should be created");
        let mut provider = provider(&root);
        provider.wait_for_scan();

        assert_eq!(provider.search("old").len(), 1);

        fs::write(root.join("new-notes.md"), "hello").expect("new file should be created");
        provider.refresh().expect("refresh should start a scan");
        provider.wait_for_scan();

        assert_eq!(provider.search("new").len(), 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn searches_binary_document_files() {
        let root = temp_dir("binary-docs");
        fs::create_dir_all(&root).expect("temp directory should be created");
        let file = root.join("The Rust Programming Language.pdf");
        fs::write(&file, "%PDF").expect("file should be created");
        let provider = provider(&root);
        provider.wait_for_scan();

        let results = provider.search("rust programming");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].provider, "file_search");
        assert_eq!(results[0].id, file.to_string_lossy());
        assert_eq!(results[0].title, "The Rust Programming Language.pdf");
        assert!(
            !results[0]
                .actions
                .iter()
                .any(|action| action.id == "open_editor")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scores_are_normalized_across_roots() {
        let parent = temp_dir("global-score");
        let books = parent.join("books");
        let papers = parent.join("papers");
        fs::create_dir_all(&books).expect("books directory should be created");
        fs::create_dir_all(&papers).expect("papers directory should be created");
        let stronger_match = books.join("rust.pdf");
        let weaker_match = papers.join("r-u-s-t-notes-and-other-material.pdf");
        fs::write(&stronger_match, "%PDF").expect("books file should be created");
        fs::write(&weaker_match, "%PDF").expect("papers file should be created");
        let provider = FileSearchProvider::new(
            &FileSearchProviderConfig {
                enabled: true,
                roots: vec![
                    books.to_string_lossy().to_string(),
                    papers.to_string_lossy().to_string(),
                ],
                min_query_len: 3,
                max_results: 50,
            },
            &FilesProviderConfig {
                enabled: true,
                editor_command: String::new(),
                open_command: "xdg-open".to_string(),
            },
        );
        provider.wait_for_scan();

        let results = provider.search("rust");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].score, 1.0);
        assert!(results[1].score < 1.0);
        assert!(
            results
                .iter()
                .any(|result| result.id == stronger_match.to_string_lossy())
        );
        assert!(
            results
                .iter()
                .any(|result| result.id == weaker_match.to_string_lossy())
        );

        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn root_alias_prefix_scopes_search_to_that_root() {
        let parent = temp_dir("scoped");
        let books = parent.join("books");
        let papers = parent.join("papers");
        fs::create_dir_all(&books).expect("books directory should be created");
        fs::create_dir_all(&papers).expect("papers directory should be created");
        let books_file = books.join("rust-notes.pdf");
        let papers_file = papers.join("rust-notes.pdf");
        fs::write(&books_file, "%PDF").expect("books file should be created");
        fs::write(&papers_file, "%PDF").expect("papers file should be created");
        let provider = FileSearchProvider::new(
            &FileSearchProviderConfig {
                enabled: true,
                roots: vec![
                    books.to_string_lossy().to_string(),
                    papers.to_string_lossy().to_string(),
                ],
                min_query_len: 3,
                max_results: 50,
            },
            &FilesProviderConfig {
                enabled: true,
                editor_command: String::new(),
                open_command: "xdg-open".to_string(),
            },
        );
        provider.wait_for_scan();

        let results = provider.search("books rust");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, books_file.to_string_lossy());

        let _ = fs::remove_dir_all(parent);
    }

    #[test]
    fn unknown_root_alias_searches_all_roots() {
        let parent = temp_dir("unknown-scope");
        let books = parent.join("books");
        let papers = parent.join("papers");
        fs::create_dir_all(&books).expect("books directory should be created");
        fs::create_dir_all(&papers).expect("papers directory should be created");
        fs::write(books.join("rust-notes.pdf"), "%PDF").expect("books file should be created");
        fs::write(papers.join("rust-notes.pdf"), "%PDF").expect("papers file should be created");
        let provider = FileSearchProvider::new(
            &FileSearchProviderConfig {
                enabled: true,
                roots: vec![
                    books.to_string_lossy().to_string(),
                    papers.to_string_lossy().to_string(),
                ],
                min_query_len: 3,
                max_results: 50,
            },
            &FilesProviderConfig {
                enabled: true,
                editor_command: String::new(),
                open_command: "xdg-open".to_string(),
            },
        );
        provider.wait_for_scan();

        let results = provider.search("rust");

        assert_eq!(results.len(), 2);

        let _ = fs::remove_dir_all(parent);
    }
}
