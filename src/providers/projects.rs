use std::{
    fs,
    os::unix::fs::FileTypeExt,
    path::{
        Path,
        PathBuf,
    },
    process::{
        Command,
        Output,
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

#[derive(Clone, Debug)]
struct Project {
    path: PathBuf,
    title: String,
    parent: String,
}

pub struct ProjectsProvider {
    roots: Vec<String>,
    kitty_command: String,
    kitty_remote: String,
    projects: Vec<Project>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectsProviderConfig {
    pub enabled: bool,
    pub roots: Vec<String>,
    pub kitty_command: String,
    pub kitty_remote: String,
}

impl Default for ProjectsProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            roots: vec!["~/dev/projects".to_string()],
            kitty_command: "kitty".to_string(),
            kitty_remote: "auto".to_string(),
        }
    }
}

impl ProjectsProvider {
    pub fn new(config: &ProjectsProviderConfig) -> Self {
        let projects = discover_projects(&config.roots);

        Self {
            roots: config.roots.clone(),
            kitty_command: config.kitty_command.clone(),
            kitty_remote: config.kitty_remote.clone(),
            projects,
        }
    }
}

impl Provider for ProjectsProvider {
    fn id(&self) -> &'static str {
        "projects"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.to_lowercase();
        let mut results = vec![];

        for project in &self.projects {
            let title = project.title.to_lowercase();
            let path = project.path.to_string_lossy().to_lowercase();
            let score = if query.is_empty() {
                1.0
            } else if title.contains(&query) {
                1.0
            } else if path.contains(&query) {
                0.5
            } else {
                -1.0
            };

            if score > 0.0 {
                results.push(SearchResult {
                    id: project.path.to_string_lossy().to_string(),
                    provider: self.id(),
                    kind: ResultKind::Project,
                    section: "Projects".to_string(),
                    title: project.title.clone(),
                    subtitle: project.parent.clone(),
                    icon: "builtin:terminal".to_string(),
                    score,
                    default_action: "open_terminal".to_string(),
                    actions: vec![
                        SearchAction::new("open_terminal", "Open", "builtin:terminal"),
                        SearchAction::new("copy_path", "Copy Path", "").immediate(),
                    ],
                    autocomplete: None,
                });
            }
        }

        results.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.title.cmp(&b.title))
                .then_with(|| a.id.cmp(&b.id))
        });

        results
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        let Some(project) = self
            .projects
            .iter()
            .find(|project| project.path == Path::new(id))
        else {
            bail!("project not found: {id}");
        };

        match action {
            "open_terminal" => {
                open_in_kitty(&self.kitty_command, &self.kitty_remote, &project.path)
            }
            "copy_path" => clipboard::copy_text(&project.path.to_string_lossy()),
            _ => bail!("unsupported project action: {action}"),
        }
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        self.projects = discover_projects(&self.roots);
        Ok(())
    }
}

fn discover_projects(roots: &[String]) -> Vec<Project> {
    let mut projects = vec![];

    for root in roots {
        let root_path = expand_home(root);
        let entries = match fs::read_dir(&root_path) {
            Ok(entries) => entries,
            Err(err) => {
                eprintln!("failed to read project root {}: {err}", root_path.display());
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    eprintln!(
                        "failed to read project entry under {}: {err}",
                        root_path.display()
                    );
                    continue;
                }
            };

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(err) => {
                    eprintln!(
                        "failed to read project entry type {}: {err}",
                        entry.path().display()
                    );
                    continue;
                }
            };

            if !file_type.is_dir() {
                continue;
            }

            let path = match entry.path().canonicalize() {
                Ok(path) => path,
                Err(err) => {
                    eprintln!(
                        "failed to canonicalize project path {}: {err}",
                        entry.path().display()
                    );
                    continue;
                }
            };

            let Some(title) = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|title| title.to_string())
            else {
                continue;
            };
            let parent = path
                .parent()
                .map(|parent| parent.to_string_lossy().to_string())
                .unwrap_or_default();

            projects.push(Project {
                path,
                title,
                parent,
            });
        }
    }

    projects.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.path.cmp(&b.path)));
    projects
}

fn expand_home(path: &str) -> PathBuf {
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

fn open_in_kitty(kitty_command: &str, kitty_remote: &str, path: &Path) -> anyhow::Result<()> {
    let Some(remote) = resolve_kitty_remote(kitty_command, kitty_remote)? else {
        return open_kitty_window(kitty_command, path);
    };

    match open_kitty_tab(kitty_command, &remote, path) {
        Ok(()) => Ok(()),
        Err(err) if kitty_remote == "auto" => {
            eprintln!("kitty remote launch failed, falling back to a new window: {err}");
            open_kitty_window(kitty_command, path)
        }
        Err(err) => Err(err),
    }
}

fn open_kitty_tab(kitty_command: &str, remote: &str, path: &Path) -> anyhow::Result<()> {
    let output = Command::new(kitty_command)
        .args([
            "@",
            "--to",
            remote,
            "launch",
            "--type=tab",
            "--no-response",
            "--cwd",
        ])
        .arg(path)
        .output()
        .context("while attempting to launch kitty project tab")?;

    if output.status.success() {
        Ok(())
    } else {
        bail!(
            "kitty remote launch failed: {}",
            command_output_message(&output)
        )
    }
}

fn open_kitty_window(kitty_command: &str, path: &Path) -> anyhow::Result<()> {
    let mut child = Command::new(kitty_command)
        .arg("--working-directory")
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("while attempting to spawn kitty project window")?;

    match child
        .try_wait()
        .context("while attempting to check kitty project window")?
    {
        Some(status) if status.success() => Ok(()),
        Some(status) => bail!("kitty project window exited immediately with {status}"),
        None => {
            thread::spawn(move || {
                if let Err(err) = child.wait() {
                    eprintln!("failed to reap kitty project window: {err}");
                }
            });
            Ok(())
        }
    }
}

fn resolve_kitty_remote(kitty_command: &str, kitty_remote: &str) -> anyhow::Result<Option<String>> {
    if kitty_remote != "auto" {
        if kitty_remote.is_empty() {
            bail!("kitty_remote is empty");
        }

        return Ok(Some(kitty_remote.to_string()));
    }

    let mut candidates = vec![];
    let entries =
        fs::read_dir("/tmp").context("while attempting to read /tmp for kitty sockets")?;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("failed to read /tmp entry while finding kitty socket: {err}");
                continue;
            }
        };

        let Some(name) = entry.file_name().to_str().map(|name| name.to_string()) else {
            continue;
        };

        if !name.starts_with("kitty-") {
            continue;
        }

        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if !metadata.file_type().is_socket() {
            continue;
        }

        candidates.push(path);
    }

    candidates.sort();
    candidates.reverse();

    for path in candidates {
        let remote = format!("unix:{}", path.display());
        if kitty_remote_responds(kitty_command, &remote) {
            return Ok(Some(remote));
        }
    }

    Ok(None)
}

fn kitty_remote_responds(kitty_command: &str, remote: &str) -> bool {
    matches!(
        Command::new(kitty_command)
            .args(["@", "--to", remote, "ls"])
            .output(),
        Ok(output) if output.status.success()
    )
}

fn command_output_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return format!("{}; stderr: {stderr}", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !stdout.is_empty() {
        return format!("{}; stdout: {stdout}", output.status);
    }

    output.status.to_string()
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
        std::env::temp_dir().join(format!("rika-projects-{test_name}-{nanos}"))
    }

    #[test]
    fn default_config_uses_dev_projects_root_and_kitty() {
        let config = ProjectsProviderConfig::default();

        assert!(config.enabled);
        assert_eq!(config.roots, vec!["~/dev/projects"]);
        assert_eq!(config.kitty_command, "kitty");
        assert_eq!(config.kitty_remote, "auto");
    }

    #[test]
    fn discovers_immediate_child_directories() {
        let root = temp_dir("discover");
        fs::create_dir_all(root.join("rika/src")).expect("project directory should be created");
        fs::create_dir_all(root.join("not-a-project-nested").join("child"))
            .expect("nested directory should be created");
        fs::write(root.join("README.md"), "ignore").expect("file should be created");

        let projects = discover_projects(&[root.to_string_lossy().to_string()]);

        assert_eq!(projects.len(), 2);
        assert!(projects.iter().any(|project| project.title == "rika"));
        assert!(
            projects
                .iter()
                .any(|project| project.title == "not-a-project-nested")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn search_returns_empty_query_and_typed_matches() {
        let root = temp_dir("search");
        fs::create_dir_all(root.join("rika")).expect("project directory should be created");
        fs::create_dir_all(root.join("codex")).expect("project directory should be created");
        let provider = ProjectsProvider::new(&ProjectsProviderConfig {
            enabled: true,
            roots: vec![root.to_string_lossy().to_string()],
            kitty_command: "kitty".to_string(),
            kitty_remote: "auto".to_string(),
        });

        let empty_results = provider.search("");
        let typed_results = provider.search("cod");

        assert_eq!(empty_results.len(), 2);
        assert_eq!(typed_results.len(), 1);
        assert_eq!(typed_results[0].title, "codex");
        assert_eq!(typed_results[0].kind, ResultKind::Project);
        assert_eq!(typed_results[0].section, "Projects");
        assert!(
            typed_results[0]
                .actions
                .iter()
                .any(|action| action.id == typed_results[0].default_action)
        );
        assert!(
            typed_results[0]
                .actions
                .iter()
                .any(|action| action.id == "copy_path")
        );
        assert!(
            typed_results[0]
                .actions
                .iter()
                .any(|action| action.id == "copy_path"
                    && action.close_behavior == SearchActionCloseBehavior::Immediate)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_roots_return_no_projects() {
        let root = temp_dir("missing");

        let projects = discover_projects(&[root.to_string_lossy().to_string()]);

        assert!(projects.is_empty());
    }

    #[test]
    fn explicit_kitty_remote_is_used_as_is() {
        let remote =
            resolve_kitty_remote("kitty", "unix:/tmp/kitty-123").expect("remote should resolve");

        assert_eq!(remote.as_deref(), Some("unix:/tmp/kitty-123"));
    }

    #[test]
    fn empty_kitty_remote_is_rejected() {
        let err = resolve_kitty_remote("kitty", "").expect_err("empty remote should fail");

        assert_eq!(err.to_string(), "kitty_remote is empty");
    }
}
