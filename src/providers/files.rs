use std::{
    ffi::OsStr,
    path::{
        Path,
        PathBuf,
    },
    process::{
        Child,
        Command,
        Stdio,
    },
    thread,
};

use anyhow::{
    Context,
    bail,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    clipboard,
    provider::{
        Provider,
        ResultKind,
        SearchAction,
        SearchResult,
    },
};

pub struct FilesProvider {
    editor_command: String,
    open_command: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilesProviderConfig {
    pub enabled: bool,
    pub editor_command: String,
    pub open_command: String,
}

impl Default for FilesProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            editor_command: String::new(),
            open_command: "xdg-open".to_string(),
        }
    }
}

impl FilesProvider {
    pub fn new(config: &FilesProviderConfig) -> Self {
        Self {
            editor_command: config.editor_command.clone(),
            open_command: config.open_command.clone(),
        }
    }
}

impl Provider for FilesProvider {
    fn id(&self) -> &'static str {
        "files"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let Some(path) = direct_path(query) else {
            return vec![];
        };

        let path = match path.canonicalize() {
            Ok(path) => path,
            Err(_) => return vec![],
        };

        let Ok(metadata) = path.metadata() else {
            return vec![];
        };

        vec![file_result(
            self.id(),
            &path,
            metadata.is_dir(),
            !self.editor_command.is_empty(),
        )]
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        activate_file(id, action, &self.editor_command, &self.open_command)
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub(crate) fn activate_file(
    id: &str,
    action: &str,
    editor_command: &str,
    open_command: &str,
) -> anyhow::Result<()> {
    let path = Path::new(id);
    match action {
        "open" => {
            if path.is_file() && is_text_file(path) && !editor_command.is_empty() {
                spawn_configured_command(editor_command, path)
            } else {
                spawn_configured_command(open_command, path)
            }
        }
        "open_default" => spawn_configured_command(open_command, path),
        "open_editor" => {
            if editor_command.is_empty() {
                bail!("editor_command is empty");
            }

            spawn_configured_command(editor_command, path)
        }
        "copy_path" => clipboard::copy_text(id),
        _ => bail!("unsupported file action: {action}"),
    }
}

pub(crate) fn file_result(
    provider: &'static str,
    path: &Path,
    is_dir: bool,
    has_editor: bool,
) -> SearchResult {
    file_result_with_subtitle(provider, path, is_dir, has_editor, None, 1.0)
}

pub(crate) fn file_result_with_subtitle(
    provider: &'static str,
    path: &Path,
    is_dir: bool,
    has_editor: bool,
    subtitle: Option<String>,
    score: f32,
) -> SearchResult {
    let title = path
        .file_name()
        .and_then(OsStr::to_str)
        .map(str::to_string)
        .unwrap_or_else(|| path.display().to_string());
    let subtitle = subtitle.unwrap_or_else(|| {
        path.parent()
            .map(|parent| parent.to_string_lossy().to_string())
            .unwrap_or_default()
    });
    let is_text = path.is_file() && is_text_file(path);
    let mut actions = vec![
        SearchAction::new("open", "Open", "builtin:folder"),
        SearchAction::new("open_default", "Open With Default App", ""),
        SearchAction::new("copy_path", "Copy Path", "").immediate(),
    ];

    if is_text && has_editor {
        actions.insert(1, SearchAction::new("open_editor", "Open in Editor", ""));
    }

    SearchResult {
        id: path.to_string_lossy().to_string(),
        provider,
        kind: if is_dir {
            ResultKind::Directory
        } else {
            ResultKind::File
        },
        section: "Files".to_string(),
        title,
        subtitle,
        icon: if is_dir {
            "builtin:folder".to_string()
        } else {
            file_type_icon(path).to_string()
        },
        score,
        default_action: "open".to_string(),
        actions,
        autocomplete: None,
    }
}

fn file_type_icon(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "avif"
        | "heic" | "heif" => "builtin:file-image",
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "aac" | "opus" | "aiff" => "builtin:file-audio",
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "flv" | "wmv" | "m4v" => "builtin:file-video",
        "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" | "tgz" | "tbz2" => {
            "builtin:file-archive"
        }
        "pdf" => "builtin:file-pdf",
        "txt" | "md" | "rst" | "org" | "tex" | "epub" => "builtin:file-text",
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "go" | "c" | "cpp" | "h" | "hpp"
        | "java" | "kt" | "swift" | "rb" | "php" | "cs" | "lua" | "sh" | "bash" | "fish"
        | "zsh" | "nix" | "toml" | "yaml" | "yml" | "json" | "xml" | "html" | "css" | "scss"
        | "sass" | "sql" | "qml" | "ini" | "conf" | "cfg" | "csv" => "builtin:file-code",
        _ => "builtin:file",
    }
}

fn direct_path(query: &str) -> Option<PathBuf> {
    let query = query.trim();
    if query.is_empty() || !looks_path_like(query) {
        return None;
    }

    Some(expand_path(query))
}

fn looks_path_like(query: &str) -> bool {
    query == "~"
        || query.starts_with("~/")
        || query.starts_with('/')
        || query.starts_with("./")
        || query.starts_with("../")
}

pub(crate) fn expand_path(path: &str) -> PathBuf {
    if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }

    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }

    PathBuf::from(path)
}

fn is_text_file(path: &Path) -> bool {
    has_text_extension(path)
}

fn has_text_extension(path: &Path) -> bool {
    let Some(extension) = path
        .extension()
        .and_then(OsStr::to_str)
        .map(|extension| extension.to_lowercase())
    else {
        return false;
    };

    matches!(
        extension.as_str(),
        "bash"
            | "c"
            | "cfg"
            | "conf"
            | "cpp"
            | "css"
            | "csv"
            | "fish"
            | "go"
            | "h"
            | "hpp"
            | "html"
            | "ini"
            | "java"
            | "js"
            | "json"
            | "jsx"
            | "lua"
            | "md"
            | "nix"
            | "py"
            | "qml"
            | "rs"
            | "scss"
            | "sh"
            | "sql"
            | "toml"
            | "ts"
            | "tsx"
            | "txt"
            | "xml"
            | "yaml"
            | "yml"
            | "zsh"
    )
}

fn spawn_configured_command(command: &str, path: &Path) -> anyhow::Result<()> {
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        bail!("command is empty");
    };

    let mut child = Command::new(program)
        .args(parts)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("while attempting to spawn file command")?;

    match child
        .try_wait()
        .context("while attempting to check file command")?
    {
        Some(status) if status.success() => Ok(()),
        Some(status) => bail!("file command exited immediately with {status}"),
        None => {
            reap_child(child);
            Ok(())
        }
    }
}

fn reap_child(mut child: Child) {
    thread::spawn(move || {
        if let Err(err) = child.wait() {
            eprintln!("failed to reap file command: {err}");
        }
    });
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use super::*;
    use crate::provider::SearchActionCloseBehavior;

    fn temp_dir(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("rika-files-{test_name}-{nanos}"))
    }

    #[test]
    fn default_config_uses_open_command_and_no_implicit_editor() {
        let config = FilesProviderConfig::default();

        assert!(config.enabled);
        assert!(config.editor_command.is_empty());
        assert_eq!(config.open_command, "xdg-open");
    }

    #[test]
    fn ignores_non_path_queries() {
        let provider = FilesProvider::new(&FilesProviderConfig {
            enabled: true,
            editor_command: String::new(),
            open_command: "xdg-open".to_string(),
        });

        assert!(provider.search("rika").is_empty());
    }

    #[test]
    fn returns_existing_direct_file_path() {
        let root = temp_dir("direct-file");
        fs::create_dir_all(&root).expect("temp directory should be created");
        let file = root.join("notes.md");
        fs::write(&file, "hello").expect("file should be created");
        let provider = FilesProvider::new(&FilesProviderConfig {
            enabled: true,
            editor_command: "nvim".to_string(),
            open_command: "xdg-open".to_string(),
        });

        let results = provider.search(&file.to_string_lossy());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, ResultKind::File);
        assert_eq!(results[0].section, "Files");
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
                .any(|action| action.id == "copy_path"
                    && action.close_behavior == SearchActionCloseBehavior::Immediate)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn direct_pdf_file_does_not_get_editor_action() {
        let root = temp_dir("direct-pdf");
        fs::create_dir_all(&root).expect("temp directory should be created");
        let file = root.join("notes.pdf");
        fs::write(&file, "%PDF").expect("file should be created");
        let provider = FilesProvider::new(&FilesProviderConfig {
            enabled: true,
            editor_command: "nvim".to_string(),
            open_command: "xdg-open".to_string(),
        });

        let results = provider.search(&file.to_string_lossy());

        assert_eq!(results.len(), 1);
        assert!(
            !results[0]
                .actions
                .iter()
                .any(|action| action.id == "open_editor")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn extensionless_text_file_does_not_get_editor_action() {
        let root = temp_dir("extensionless-text");
        fs::create_dir_all(&root).expect("temp directory should be created");
        let file = root.join("notes");
        fs::write(&file, "hello").expect("file should be created");
        let provider = FilesProvider::new(&FilesProviderConfig {
            enabled: true,
            editor_command: "nvim".to_string(),
            open_command: "xdg-open".to_string(),
        });

        let results = provider.search(&file.to_string_lossy());

        assert_eq!(results.len(), 1);
        assert!(
            !results[0]
                .actions
                .iter()
                .any(|action| action.id == "open_editor")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn returns_existing_direct_directory_path() {
        let root = temp_dir("direct-dir");
        fs::create_dir_all(&root).expect("temp directory should be created");
        let provider = FilesProvider::new(&FilesProviderConfig {
            enabled: true,
            editor_command: "nvim".to_string(),
            open_command: "xdg-open".to_string(),
        });

        let results = provider.search(&root.to_string_lossy());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, ResultKind::Directory);
        assert_eq!(results[0].default_action, "open");
        assert!(
            !results[0]
                .actions
                .iter()
                .any(|action| action.id == "open_editor")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_direct_path_returns_no_results() {
        let root = temp_dir("missing");
        let provider = FilesProvider::new(&FilesProviderConfig {
            enabled: true,
            editor_command: String::new(),
            open_command: "xdg-open".to_string(),
        });

        assert!(provider.search(&root.to_string_lossy()).is_empty());
    }
}
