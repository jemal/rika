use std::collections::HashSet;

use freedesktop_desktop_entry::{
    DesktopEntry,
    Iter,
    default_paths,
    get_languages_from_env,
};

use crate::provider::{
    Provider,
    SearchResult,
};

#[derive(Debug)]
pub struct DesktopApp {
    file_name: String,
    name: String,
    entry: DesktopEntry,
}

pub struct DesktopProvider {
    apps: Vec<DesktopApp>,
}

impl DesktopProvider {
    pub fn new() -> Self {
        let locales = get_languages_from_env();
        let mut seen = HashSet::new();
        let mut apps = Vec::new();

        for entry in Iter::new(default_paths()).entries(Some(&locales)) {
            if entry.hidden() || entry.no_display() {
                continue;
            }

            if !seen.insert(entry.id().to_string()) {
                continue;
            }

            let Some(file_name) = entry.path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            let Some(name) = entry.name(&locales) else {
                continue;
            };

            apps.push(DesktopApp {
                file_name: file_name.to_string(),
                name: name.to_string(),
                entry,
            });
        }

        Self { apps }
    }
}

impl Provider for DesktopProvider {
    fn id(&self) -> &'static str {
        "desktop"
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

    fn activate(&self, id: &str, action: &str) {
        match action {
            "open" => {
                let Some(app) = self.apps.iter().find(|app| app.file_name == id) else {
                    return;
                };

                let Ok(argv) = app.entry.parse_exec() else {
                    return;
                };

                if let Some((program, args)) = argv.split_first() {
                    std::process::Command::new(program).args(args).spawn().ok();
                }
            }
            _ => todo!(),
        }
    }
}
