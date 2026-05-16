use std::{
    collections::HashSet,
    process::Command,
    thread,
};

use anyhow::{
    Context,
    bail,
};
use freedesktop_desktop_entry::{
    DesktopEntry,
    Iter,
    default_paths,
    get_languages_from_env,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::provider::{
    Provider,
    SearchResult,
};

#[derive(Debug)]
pub struct App {
    file_name: String,
    name: String,
    entry: DesktopEntry,
}

pub struct AppsProvider {
    apps: Vec<App>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppsProviderConfig {
    pub enabled: bool,
}

impl Default for AppsProviderConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl AppsProvider {
    pub fn new() -> Self {
        let apps = Self::build_apps();

        Self { apps }
    }

    fn build_apps() -> Vec<App> {
        let locales = get_languages_from_env();
        let mut seen = HashSet::new();
        let mut apps = vec![];

        for entry in Iter::new(default_paths()).entries(Some(&locales)) {
            if !seen.insert(entry.id().to_string()) {
                continue;
            }

            if entry.hidden() || entry.no_display() {
                continue;
            }

            let Some(file_name) = entry.path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            let Some(name) = entry.name(&locales) else {
                continue;
            };

            apps.push(App {
                file_name: file_name.to_string(),
                name: name.to_string(),
                entry,
            });
        }

        apps
    }
}

impl Provider for AppsProvider {
    fn id(&self) -> &'static str {
        "apps"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results = vec![];
        let query = query.to_lowercase();

        for app in &self.apps {
            let name = app.name.to_lowercase();
            let id = app.entry.id().to_lowercase();

            let score = {
                if name.contains(&query) {
                    1.0
                } else if id.contains(&query) {
                    0.5
                } else {
                    -1.0
                }
            };

            if score > 0.0 {
                results.push(SearchResult {
                    id: app.file_name.clone(),
                    provider: self.id(),
                    title: app.name.to_string(),
                    subtitle: String::new(),
                    icon: app
                        .entry
                        .icon()
                        .unwrap_or("application-x-executable")
                        .to_string(),
                    score,
                    actions: vec!["open".to_string()],
                });
            }
        }

        // highest score first
        results.sort_by(|a, b| b.score.total_cmp(&a.score));

        results
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        match action {
            "open" => {
                let Some(app) = self.apps.iter().find(|app| app.file_name == id) else {
                    bail!("desktop app not found: {id}");
                };

                let argv = app
                    .entry
                    .parse_exec()
                    .context("while attempting to parse desktop entry exec")?;

                let Some((program, args)) = argv.split_first() else {
                    bail!("desktop entry exec is empty: {id}");
                };

                let mut child = Command::new(program)
                    .args(args)
                    .spawn()
                    .context("while attempting to spawn desktop app")?;

                thread::spawn(move || {
                    if let Err(err) = child.wait() {
                        eprintln!("failed to reap desktop app: {err}");
                    }
                });

                Ok(())
            }
            _ => bail!("unsupported desktop action: {action}"),
        }
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        self.apps = Self::build_apps();
        Ok(())
    }
}
