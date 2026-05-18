use std::{
    fs,
    process::Command as StdCommand,
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

use crate::provider::{
    Provider,
    SearchResult,
};

include!(concat!(env!("OUT_DIR"), "/builtin_bangs.rs"));

pub struct WebSearchProvider {
    bangs: Vec<Bang>,
    browser_command: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WebSearchProviderConfig {
    pub enabled: bool,
    pub browser_command: String,
}

struct Bang {
    name: String,
    alias: String,
    url: String,
}

#[derive(Deserialize)]
struct KagiBang {
    s: String,
    t: String,
    u: String,
}

impl Default for WebSearchProviderConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            browser_command: String::new(),
        }
    }
}

impl WebSearchProvider {
    pub fn new(config: &WebSearchProviderConfig) -> Self {
        let mut bangs: Vec<Bang> = BUILTIN_BANGS
            .iter()
            .map(|(name, alias, url)| Bang {
                name: name.to_string(),
                alias: alias.to_string(),
                url: url.to_string(),
            })
            .collect();

        if let Some(user_bangs) = load_user_bangs() {
            for user_bang in user_bangs {
                match bangs.iter_mut().find(|b| b.alias == user_bang.alias) {
                    Some(existing) => *existing = user_bang,
                    None => bangs.push(user_bang),
                }
            }
        }

        bangs.sort_by_key(|b| b.alias.len());

        Self {
            bangs,
            browser_command: config.browser_command.clone(),
        }
    }
}

fn load_user_bangs() -> Option<Vec<Bang>> {
    let path = dirs::config_dir()?.join("rika").join("bangs.json");

    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            eprintln!("failed to read {}: {e}", path.display());
            return None;
        }
    };

    match serde_json::from_str::<Vec<KagiBang>>(&content) {
        Ok(kagi_bangs) => Some(
            kagi_bangs
                .into_iter()
                .map(|b| Bang {
                    name: b.s,
                    alias: format!("!{}", b.t),
                    url: b.u,
                })
                .collect(),
        ),
        Err(e) => {
            eprintln!("failed to parse {}: {e}", path.display());
            None
        }
    }
}

impl Provider for WebSearchProvider {
    fn id(&self) -> &'static str {
        "web_search"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results = vec![];
        let query = query.to_lowercase();

        match query.split_once(' ') {
            None => {
                for bang in self
                    .bangs
                    .iter()
                    .filter(|b| b.alias.starts_with(query.as_str()))
                    .take(20)
                {
                    results.push(SearchResult {
                        id: bang.alias.clone(),
                        provider: self.id(),
                        title: format!("Search {}", bang.name),
                        subtitle: String::new(),
                        icon: "builtin:globe".to_string(),
                        score: 1.0,
                        actions: vec!["noop".to_string()],
                        autocomplete: Some(bang.alias.clone()),
                    });
                }
            }
            Some((alias, search_query)) => {
                if let Some(bang) = self.bangs.iter().find(|b| b.alias == alias) {
                    let search_query = search_query.trim();
                    let (id, title, actions) = if search_query.is_empty() {
                        (
                            alias.to_string(),
                            format!("Search {}", bang.name),
                            vec!["noop".to_string()],
                        )
                    } else {
                        (
                            format!("{alias}:{search_query}"),
                            format!("Search {} - {search_query}", bang.name),
                            vec!["search".to_string()],
                        )
                    };

                    results.push(SearchResult {
                        id,
                        provider: self.id(),
                        title,
                        subtitle: String::new(),
                        icon: "builtin:globe".to_string(),
                        score: 1.0,
                        actions,
                        autocomplete: None,
                    });
                }
            }
        }

        results
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        match action {
            "search" => {
                let Some((alias, query)) = id.split_once(':') else {
                    bail!("invalid web_search id: {id}");
                };

                let Some(bang) = self.bangs.iter().find(|bang| bang.alias == alias) else {
                    bail!("unknown bang: {alias}");
                };

                let constructed_url = bang.url.replace("{{{s}}}", query);

                let mut parts = self.browser_command.split_whitespace();
                let Some(cmd) = parts.next() else {
                    bail!("browser_command is empty");
                };

                let mut child = StdCommand::new(cmd)
                    .args(parts)
                    .arg(constructed_url)
                    .spawn()
                    .context("while attempting to spawn browser")?;

                thread::spawn(move || {
                    if let Err(err) = child.wait() {
                        eprintln!("failed to reap command: {err}");
                    }
                });

                Ok(())
            }
            "noop" => Ok(()),
            _ => bail!("unsupported web_search action: {action}"),
        }
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
