use std::{
    collections::HashMap,
    fs,
    path::{
        Path,
        PathBuf,
    },
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::Context;
use serde::{
    Deserialize,
    Serialize,
};

use crate::provider::SearchResult;

const USAGE_BOOST_SCALE: f32 = 0.15;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct UsageKey {
    provider: String,
    id: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct UsageRecord {
    provider: String,
    id: String,
    activation_count: u64,
    last_used_at_unix: u64,
}

#[derive(Default, Deserialize, Serialize)]
struct UsageState {
    entries: Vec<UsageRecord>,
}

#[derive(Default)]
pub struct UsageStore {
    path: Option<PathBuf>,
    records: HashMap<UsageKey, UsageRecord>,
}

impl UsageStore {
    pub fn load() -> Self {
        let Some(path) = state_file_path() else {
            eprintln!("failed to resolve state directory for usage ranking");
            return Self::default();
        };

        Self::load_from_path(path)
    }

    fn load_from_path(path: PathBuf) -> Self {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Self {
                    path: Some(path),
                    records: HashMap::new(),
                };
            }
            Err(err) => {
                eprintln!("failed to read {}: {err}", path.display());
                return Self {
                    path: Some(path),
                    records: HashMap::new(),
                };
            }
        };

        let state = match serde_json::from_str::<UsageState>(&content) {
            Ok(state) => state,
            Err(err) => {
                eprintln!("failed to parse {}: {err}", path.display());
                return Self {
                    path: Some(path),
                    records: HashMap::new(),
                };
            }
        };

        Self {
            path: Some(path),
            records: state
                .entries
                .into_iter()
                .map(|record| {
                    (
                        UsageKey {
                            provider: record.provider.clone(),
                            id: record.id.clone(),
                        },
                        record,
                    )
                })
                .collect(),
        }
    }

    pub fn record_activation(&mut self, provider: &str, id: &str, action: &str) -> bool {
        let Some(key) = activation_usage_key(provider, id, action) else {
            return false;
        };

        let record = self.records.entry(key.clone()).or_insert(UsageRecord {
            provider: key.provider,
            id: key.id,
            activation_count: 0,
            last_used_at_unix: 0,
        });

        record.activation_count = record.activation_count.saturating_add(1);
        record.last_used_at_unix = current_unix_timestamp();

        true
    }

    pub fn boost_results(&self, results: &mut [SearchResult]) {
        for result in results {
            let key = result_usage_key(result.provider, &result.id);
            if let Some(record) = self.records.get(&key) {
                result.score += usage_boost(record.activation_count);
            }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("while attempting to create state directory")?;
        }

        let mut entries: Vec<UsageRecord> = self.records.values().cloned().collect();
        entries.sort_by(|a, b| a.provider.cmp(&b.provider).then_with(|| a.id.cmp(&b.id)));

        let content = serde_json::to_string_pretty(&UsageState { entries })
            .context("while attempting to serialize usage state")?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, content).context("while attempting to write temporary usage state")?;
        fs::rename(&tmp_path, path).context("while attempting to replace usage state")?;

        Ok(())
    }
}

pub fn sort_results(results: &mut [SearchResult]) {
    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.provider.cmp(b.provider))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
}

fn state_file_path() -> Option<PathBuf> {
    dirs::state_dir().map(|state_dir| state_file_path_from(&state_dir))
}

fn state_file_path_from(state_dir: &Path) -> PathBuf {
    state_dir.join("rika").join("state.json")
}

fn activation_usage_key(provider: &str, id: &str, action: &str) -> Option<UsageKey> {
    if action == "noop" {
        return None;
    }

    Some(result_usage_key(provider, id))
}

fn result_usage_key(provider: &str, id: &str) -> UsageKey {
    let id = if provider == "web_search" {
        id.split_once(':').map_or(id, |(alias, _)| alias)
    } else {
        id
    };

    UsageKey {
        provider: provider.to_string(),
        id: id.to_string(),
    }
}

fn usage_boost(activation_count: u64) -> f32 {
    (1.0 + activation_count as f32).ln() * USAGE_BOOST_SCALE
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("current time should be after unix epoch")
        .as_secs()
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

    fn temp_state_path(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();

        std::env::temp_dir()
            .join(format!("rika-{test_name}-{nanos}"))
            .join("state.json")
    }

    fn result(provider: &'static str, id: &str, score: f32) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            provider,
            title: id.to_string(),
            subtitle: String::new(),
            icon: String::new(),
            score,
            actions: vec!["open".to_string()],
            autocomplete: None,
        }
    }

    #[test]
    fn state_path_uses_rika_state_json_under_base_state_dir() {
        assert_eq!(
            state_file_path_from(Path::new("/tmp/state")),
            Path::new("/tmp/state").join("rika").join("state.json")
        );
    }

    #[test]
    fn activation_key_normalizes_web_search_queries() {
        let key = activation_usage_key("web_search", "!gh:rust anyhow", "search")
            .expect("search action should produce a key");

        assert_eq!(key.provider, "web_search");
        assert_eq!(key.id, "!gh");
    }

    #[test]
    fn activation_key_ignores_noop_actions() {
        assert!(activation_usage_key("web_search", "!gh", "noop").is_none());
    }

    #[test]
    fn activation_key_keeps_non_web_result_ids() {
        let key = activation_usage_key("apps", "firefox.desktop", "open")
            .expect("open action should produce a key");

        assert_eq!(key.provider, "apps");
        assert_eq!(key.id, "firefox.desktop");
    }

    #[test]
    fn boost_results_uses_canonical_web_bang_key() {
        let mut store = UsageStore::default();
        assert!(store.record_activation("web_search", "!gh:rust", "search"));
        assert!(store.record_activation("web_search", "!gh:serde", "search"));

        let mut results = vec![
            result("web_search", "!yt", 1.0),
            result("web_search", "!gh", 1.0),
        ];

        store.boost_results(&mut results);
        results.sort_by(|a, b| b.score.total_cmp(&a.score));

        assert_eq!(results[0].id, "!gh");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn sort_results_breaks_equal_score_ties_stably() {
        let mut results = vec![
            result("web_search", "!gh", 1.0),
            result("apps", "z.desktop", 1.0),
            result("apps", "a.desktop", 1.0),
        ];

        results[0].title = "GitHub".to_string();
        results[1].title = "Zed".to_string();
        results[2].title = "Alpha".to_string();

        sort_results(&mut results);

        assert_eq!(results[0].id, "a.desktop");
        assert_eq!(results[1].id, "z.desktop");
        assert_eq!(results[2].id, "!gh");
    }

    #[test]
    fn save_and_load_round_trips_usage_state() {
        let path = temp_state_path("round-trip");
        let mut store = UsageStore::load_from_path(path.clone());
        assert!(store.record_activation("apps", "firefox.desktop", "open"));
        store.save().expect("usage state should save");
        assert!(!path.with_extension("json.tmp").exists());

        let loaded = UsageStore::load_from_path(path.clone());
        let key = result_usage_key("apps", "firefox.desktop");
        let record = loaded.records.get(&key).expect("saved record should load");

        assert_eq!(record.activation_count, 1);

        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn missing_state_file_starts_empty() {
        let path = temp_state_path("missing");
        let store = UsageStore::load_from_path(path);

        assert!(store.records.is_empty());
    }

    #[test]
    fn malformed_state_file_starts_empty() {
        let path = temp_state_path("malformed");
        fs::create_dir_all(path.parent().expect("state file should have parent"))
            .expect("temp dir should be created");
        fs::write(&path, "{not json").expect("malformed state should be written");

        let store = UsageStore::load_from_path(path.clone());

        assert!(store.records.is_empty());

        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }
}
