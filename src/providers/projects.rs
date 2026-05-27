use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },
    process::{
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

#[derive(Clone, Debug)]
struct Project {
    path: PathBuf,
    title: String,
    root_alias: String,
}

pub struct ProjectsProvider {
    roots: Vec<String>,
    default_action: String,
    actions: Vec<ProjectActionConfig>,
    projects: Vec<Project>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectsProviderConfig {
    pub enabled: bool,
    pub roots: Vec<String>,
    pub default_action: String,
    pub actions: Vec<ProjectActionConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectActionConfig {
    id: String,
    label: String,
    command: String,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    icon: String,
    #[serde(default = "default_project_action_cwd")]
    cwd: String,
}

impl Default for ProjectsProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            roots: vec!["~/dev/projects".to_string()],
            default_action: "open_project".to_string(),
            actions: default_project_actions(),
        }
    }
}

impl ProjectsProvider {
    pub fn new(config: &ProjectsProviderConfig) -> Self {
        let projects = discover_projects(&config.roots);
        let default_action = valid_default_action(&config.default_action, &config.actions);

        Self {
            roots: config.roots.clone(),
            default_action,
            actions: config.actions.clone(),
            projects,
        }
    }
}

impl Provider for ProjectsProvider {
    fn id(&self) -> &'static str {
        "projects"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim();
        if query.is_empty() {
            return vec![];
        }

        let (candidates, effective_query) = scoped_query(&self.projects, query);
        let effective_query = effective_query.to_lowercase();
        let mut results = vec![];

        for project in candidates {
            let title = project.title.to_lowercase();
            let path = project.path.to_string_lossy().to_lowercase();
            let score = if title.contains(&effective_query) {
                1.0
            } else if path.contains(&effective_query) {
                0.5
            } else {
                -1.0
            };

            if score > 0.0 {
                let mut actions: Vec<SearchAction> = self
                    .actions
                    .iter()
                    .map(|action| SearchAction::new(&action.id, &action.label, &action.icon))
                    .collect();
                actions.push(SearchAction::new("copy_path", "Copy Path", "").immediate());

                results.push(SearchResult {
                    id: project.path.to_string_lossy().to_string(),
                    provider: self.id(),
                    kind: ResultKind::Project,
                    section: "Projects".to_string(),
                    title: project.title.clone(),
                    subtitle: project.root_alias.clone(),
                    icon: "builtin:folder-git-2".to_string(),
                    score,
                    default_action: self.default_action.clone(),
                    actions,
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
            "copy_path" => clipboard::copy_text(&project.path.to_string_lossy()),
            action_id => {
                let Some(action) = self.actions.iter().find(|action| action.id == action_id) else {
                    bail!("unsupported project action: {action_id}");
                };

                run_project_action(action, project)
            }
        }
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        self.projects = discover_projects(&self.roots);
        Ok(())
    }
}

fn default_project_actions() -> Vec<ProjectActionConfig> {
    vec![ProjectActionConfig {
        id: "open_project".to_string(),
        label: "Open".to_string(),
        command: "xdg-open".to_string(),
        args: vec!["{{path}}".to_string()],
        icon: "builtin:folder".to_string(),
        cwd: String::new(),
    }]
}

fn default_project_action_cwd() -> String {
    "{{path}}".to_string()
}

fn valid_default_action(default_action: &str, actions: &[ProjectActionConfig]) -> String {
    if default_action == "copy_path" || actions.iter().any(|action| action.id == default_action) {
        return default_action.to_string();
    }

    actions
        .first()
        .map(|action| action.id.clone())
        .unwrap_or_else(|| "copy_path".to_string())
}

fn scoped_query<'a>(projects: &'a [Project], query: &'a str) -> (Vec<&'a Project>, &'a str) {
    let Some((first, rest)) = query.split_once(char::is_whitespace) else {
        return (projects.iter().collect(), query);
    };

    let rest = rest.trim_start();
    if rest.is_empty() {
        return (projects.iter().collect(), query);
    }

    let matches: Vec<&Project> = projects
        .iter()
        .filter(|p| p.root_alias.eq_ignore_ascii_case(first))
        .collect();

    if matches.is_empty() {
        (projects.iter().collect(), query)
    } else {
        (matches, rest)
    }
}

fn discover_projects(roots: &[String]) -> Vec<Project> {
    let mut projects = vec![];

    for root in roots {
        let root_path = expand_home(root);
        let root_alias = root_path
            .canonicalize()
            .unwrap_or_else(|_| root_path.clone())
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| root_path.display().to_string());
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

            projects.push(Project {
                path,
                title,
                root_alias: root_alias.clone(),
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

fn run_project_action(action: &ProjectActionConfig, project: &Project) -> anyhow::Result<()> {
    if action.command.trim().is_empty() {
        bail!("project action command is empty: {}", action.id);
    }

    let args: Vec<String> = action
        .args
        .iter()
        .map(|arg| expand_project_template(arg, project))
        .collect();
    let cwd = expand_project_template(&action.cwd, project);
    let mut command = Command::new(&action.command);
    command.args(args);

    if !cwd.trim().is_empty() {
        command.current_dir(cwd);
    }

    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("while attempting to spawn project action {}", action.id))?;

    match child
        .try_wait()
        .with_context(|| format!("while attempting to check project action {}", action.id))?
    {
        Some(status) if status.success() => Ok(()),
        Some(status) => bail!(
            "project action {} exited immediately with {status}",
            action.id
        ),
        None => {
            thread::spawn(move || {
                if let Err(err) = child.wait() {
                    eprintln!("failed to reap project action: {err}");
                }
            });
            Ok(())
        }
    }
}

fn expand_project_template(template: &str, project: &Project) -> String {
    template
        .replace("{{path}}", &project.path.to_string_lossy())
        .replace("{{title}}", &project.title)
        .replace("{{root_alias}}", &project.root_alias)
        .replace(
            "{{session}}",
            &project_session_name(&project.path, &project.title),
        )
}

fn project_session_name(path: &Path, title: &str) -> String {
    let hash = stable_path_hash(path);
    let slug = title
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    let slug = if slug.is_empty() { "project" } else { &slug };

    format!("rika-{slug}-{hash:08x}")
}

fn stable_path_hash(path: &Path) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in path.to_string_lossy().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }

    hash
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
    fn default_config_uses_dev_projects_root_and_actions() {
        let config = ProjectsProviderConfig::default();

        assert!(config.enabled);
        assert_eq!(config.roots, vec!["~/dev/projects"]);
        assert_eq!(config.default_action, "open_project");
        assert_eq!(config.actions.len(), 1);
        assert_eq!(config.actions[0].id, "open_project");
        assert_eq!(config.actions[0].command, "xdg-open");
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
            default_action: "open_terminal".to_string(),
            actions: vec![
                ProjectActionConfig {
                    id: "open_terminal".to_string(),
                    label: "Open".to_string(),
                    command: "terminal".to_string(),
                    args: vec!["{{path}}".to_string()],
                    icon: "builtin:terminal".to_string(),
                    cwd: String::new(),
                },
                ProjectActionConfig {
                    id: "open_zellij".to_string(),
                    label: "Open in Zellij".to_string(),
                    command: "terminal".to_string(),
                    args: vec!["zellij".to_string(), "{{session}}".to_string()],
                    icon: "builtin:terminal".to_string(),
                    cwd: String::new(),
                },
            ],
        });

        let empty_results = provider.search("");
        let typed_results = provider.search("cod");

        assert!(empty_results.is_empty());
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
                .any(|action| action.id == "open_zellij")
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
    fn alias_prefix_scopes_to_matching_root() {
        let root_a = temp_dir("scope-a");
        let root_b = temp_dir("scope-b");
        fs::create_dir_all(root_a.join("shared")).expect("project directory should be created");
        fs::create_dir_all(root_b.join("shared")).expect("project directory should be created");
        fs::create_dir_all(root_a.join("unique-a")).expect("project directory should be created");
        fs::create_dir_all(root_b.join("unique-b")).expect("project directory should be created");
        let root_a_name = root_a.file_name().unwrap().to_string_lossy().to_string();
        let root_b_name = root_b.file_name().unwrap().to_string_lossy().to_string();
        let provider = ProjectsProvider::new(&ProjectsProviderConfig {
            enabled: true,
            roots: vec![
                root_a.to_string_lossy().to_string(),
                root_b.to_string_lossy().to_string(),
            ],
            default_action: "open_terminal".to_string(),
            actions: vec![],
        });

        let scoped_a = provider.search(&format!("{root_a_name} shared"));
        let scoped_b = provider.search(&format!("{root_b_name} shared"));

        assert_eq!(scoped_a.len(), 1);
        assert_eq!(scoped_a[0].title, "shared");
        assert_eq!(
            scoped_a[0].id,
            root_a
                .join("shared")
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        );

        assert_eq!(scoped_b.len(), 1);
        assert_eq!(scoped_b[0].title, "shared");
        assert_eq!(
            scoped_b[0].id,
            root_b
                .join("shared")
                .canonicalize()
                .unwrap()
                .to_string_lossy()
        );

        let _ = fs::remove_dir_all(root_a);
        let _ = fs::remove_dir_all(root_b);
    }

    #[test]
    fn unscoped_query_searches_all_roots() {
        let root_a = temp_dir("unscoped-a");
        let root_b = temp_dir("unscoped-b");
        fs::create_dir_all(root_a.join("alpha")).expect("project directory should be created");
        fs::create_dir_all(root_b.join("alpha")).expect("project directory should be created");
        let provider = ProjectsProvider::new(&ProjectsProviderConfig {
            enabled: true,
            roots: vec![
                root_a.to_string_lossy().to_string(),
                root_b.to_string_lossy().to_string(),
            ],
            default_action: "open_terminal".to_string(),
            actions: vec![],
        });

        let results = provider.search("alpha");

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.title == "alpha"));

        let _ = fs::remove_dir_all(root_a);
        let _ = fs::remove_dir_all(root_b);
    }

    #[test]
    fn non_matching_prefix_uses_full_query() {
        let root = temp_dir("fallback");
        fs::create_dir_all(root.join("myproject")).expect("project directory should be created");
        let provider = ProjectsProvider::new(&ProjectsProviderConfig {
            enabled: true,
            roots: vec![root.to_string_lossy().to_string()],
            default_action: "open_terminal".to_string(),
            actions: vec![],
        });

        // "unknown" doesn't match any root alias, so the full query "unknown myproject" is
        // searched as-is — it doesn't match the title "myproject" as a substring
        let results = provider.search("unknown myproject");
        assert!(results.is_empty());

        // without the non-matching prefix the project is found normally
        let results = provider.search("myproject");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "myproject");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scoped_query_alias_with_no_trailing_content_is_unscoped() {
        // scoped_query only scopes when there is whitespace AND non-empty remainder
        let projects = vec![Project {
            path: PathBuf::from("/tmp/root/foo"),
            title: "foo".to_string(),
            root_alias: "root".to_string(),
        }];

        // "root" alone — no whitespace, falls back to unscoped
        let (candidates, query) = scoped_query(&projects, "root");
        assert_eq!(candidates.len(), 1);
        assert_eq!(query, "root");

        // "root " — whitespace but empty rest, falls back to unscoped
        let (candidates, query) = scoped_query(&projects, "root ");
        assert_eq!(candidates.len(), 1);
        assert_eq!(query, "root ");
    }

    #[test]
    fn project_session_names_are_stable_and_path_scoped() {
        let first = project_session_name(Path::new("/tmp/projects/rika"), "rika");
        let second = project_session_name(Path::new("/tmp/other/rika"), "rika");
        let repeat = project_session_name(Path::new("/tmp/projects/rika"), "rika");

        assert_eq!(first, repeat);
        assert_ne!(first, second);
        assert!(first.starts_with("rika-rika-"));
    }

    #[test]
    fn project_action_templates_can_support_editor_commands() {
        let project = Project {
            path: PathBuf::from("/tmp/projects/rika"),
            title: "rika".to_string(),
            root_alias: "projects".to_string(),
        };

        assert_eq!(
            expand_project_template("zed {{path}}", &project),
            "zed /tmp/projects/rika"
        );
        assert_eq!(
            expand_project_template("{{title}}:{{root_alias}}:{{session}}", &project),
            format!(
                "rika:projects:{}",
                project_session_name(Path::new("/tmp/projects/rika"), "rika")
            )
        );
    }

    #[test]
    fn invalid_default_action_falls_back_to_first_configured_action() {
        let actions = vec![ProjectActionConfig {
            id: "open_zed".to_string(),
            label: "Open in Zed".to_string(),
            command: "zed".to_string(),
            args: vec!["{{path}}".to_string()],
            icon: String::new(),
            cwd: "{{path}}".to_string(),
        }];

        assert_eq!(valid_default_action("missing", &actions), "open_zed");
        assert_eq!(valid_default_action("copy_path", &[]), "copy_path");
        assert_eq!(valid_default_action("missing", &[]), "copy_path");
    }
}
