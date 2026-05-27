use std::{
    cmp::Ordering,
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
const FAVORITES_SECTION: &str = "Favorites";
const RECENT_SECTION: &str = "Recent";
pub const ADD_FAVORITE_ACTION: &str = "favorite_add";
pub const REMOVE_FAVORITE_ACTION: &str = "favorite_remove";

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
    #[serde(default)]
    entries: Vec<UsageRecord>,
    #[serde(default)]
    favorites: Vec<FavoriteRecord>,
}

#[derive(Clone, Deserialize, Serialize)]
struct FavoriteRecord {
    provider: String,
    id: String,
    #[serde(default)]
    favorited_at_unix: u64,
}

#[derive(Default)]
pub struct UsageStore {
    path: Option<PathBuf>,
    records: HashMap<UsageKey, UsageRecord>,
    favorites: HashMap<UsageKey, FavoriteRecord>,
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
                    favorites: HashMap::new(),
                };
            }
            Err(err) => {
                eprintln!("failed to read {}: {err}", path.display());
                return Self {
                    path: Some(path),
                    records: HashMap::new(),
                    favorites: HashMap::new(),
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
                    favorites: HashMap::new(),
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
            favorites: state
                .favorites
                .into_iter()
                .map(|favorite| {
                    (
                        UsageKey {
                            provider: favorite.provider.clone(),
                            id: favorite.id.clone(),
                        },
                        favorite,
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

    pub fn recent_results(&self, results: &[SearchResult], limit: usize) -> Vec<SearchResult> {
        let result_by_key: HashMap<UsageKey, &SearchResult> = results
            .iter()
            .map(|result| (result_usage_key(result.provider, &result.id), result))
            .collect();
        let mut records: Vec<&UsageRecord> = self.records.values().collect();
        records.sort_by(|a, b| {
            b.last_used_at_unix
                .cmp(&a.last_used_at_unix)
                .then_with(|| b.activation_count.cmp(&a.activation_count))
                .then_with(|| a.provider.cmp(&b.provider))
                .then_with(|| a.id.cmp(&b.id))
        });

        records
            .into_iter()
            .filter_map(|record| {
                let key = UsageKey {
                    provider: record.provider.clone(),
                    id: record.id.clone(),
                };
                result_by_key.get(&key).map(|result| {
                    let mut recent = (*result).clone();
                    recent.section = RECENT_SECTION.to_string();
                    recent
                })
            })
            .take(limit)
            .collect()
    }

    pub fn favorite_results(&self, results: &[SearchResult]) -> Vec<SearchResult> {
        let result_by_key: HashMap<UsageKey, &SearchResult> = results
            .iter()
            .map(|result| (result_usage_key(result.provider, &result.id), result))
            .collect();
        let mut favorites: Vec<(&UsageKey, &FavoriteRecord)> = self.favorites.iter().collect();
        favorites.sort_by(|(a_key, a), (b_key, b)| {
            b.favorited_at_unix
                .cmp(&a.favorited_at_unix)
                .then_with(|| a_key.provider.cmp(&b_key.provider))
                .then_with(|| a_key.id.cmp(&b_key.id))
        });

        favorites
            .into_iter()
            .filter_map(|(key, _)| {
                result_by_key.get(key).map(|result| {
                    let mut favorite = (*result).clone();
                    favorite.section = FAVORITES_SECTION.to_string();
                    favorite
                })
            })
            .collect()
    }

    pub fn add_result_actions(&self, results: &mut [SearchResult]) {
        for result in results {
            if !can_favorite_result(result) {
                continue;
            }

            let key = result_usage_key(result.provider, &result.id);
            if self.favorites.contains_key(&key) {
                result.actions.push(
                    crate::provider::SearchAction::new(
                        REMOVE_FAVORITE_ACTION,
                        "Remove from Favorites",
                        "",
                    )
                    .keep_open()
                    .success_message("Removed from Favorites"),
                );
            } else {
                result.actions.push(
                    crate::provider::SearchAction::new(ADD_FAVORITE_ACTION, "Add to Favorites", "")
                        .keep_open()
                        .success_message("Added to Favorites"),
                );
            }
        }
    }

    pub fn handle_favorite_action(&mut self, provider: &str, id: &str, action: &str) -> bool {
        let key = result_usage_key(provider, id);
        match action {
            ADD_FAVORITE_ACTION => {
                if self.favorites.contains_key(&key) {
                    false
                } else {
                    self.favorites.insert(
                        key,
                        FavoriteRecord {
                            provider: provider.to_string(),
                            id: id.to_string(),
                            favorited_at_unix: current_unix_timestamp(),
                        },
                    );
                    true
                }
            }
            REMOVE_FAVORITE_ACTION => self.favorites.remove(&key).is_some(),
            _ => false,
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

        let mut favorites: Vec<FavoriteRecord> = self.favorites.values().cloned().collect();
        favorites.sort_by(|a, b| a.provider.cmp(&b.provider).then_with(|| a.id.cmp(&b.id)));

        let content = serde_json::to_string_pretty(&UsageState { entries, favorites })
            .context("while attempting to serialize usage state")?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, content).context("while attempting to write temporary usage state")?;
        fs::rename(&tmp_path, path).context("while attempting to replace usage state")?;

        Ok(())
    }
}

pub fn sort_results(results: &mut [SearchResult]) {
    results.sort_by(|a, b| {
        section_rank(&a.section)
            .cmp(&section_rank(&b.section))
            .then_with(|| global_result_cmp(a, b))
    });
}

pub fn remove_synthetic_duplicates(results: &mut Vec<SearchResult>) {
    let favorite_keys: Vec<UsageKey> = results
        .iter()
        .filter(|result| result.section == FAVORITES_SECTION)
        .map(|result| result_usage_key(result.provider, &result.id))
        .collect();
    let recent_keys: Vec<UsageKey> = results
        .iter()
        .filter(|result| result.section == RECENT_SECTION)
        .map(|result| result_usage_key(result.provider, &result.id))
        .collect();

    results.retain(|result| {
        let key = result_usage_key(result.provider, &result.id);
        if result.section == FAVORITES_SECTION {
            return true;
        }

        if result.section == RECENT_SECTION {
            return !favorite_keys
                .iter()
                .any(|favorite_key| *favorite_key == key);
        }

        !favorite_keys
            .iter()
            .any(|favorite_key| *favorite_key == key)
            && !recent_keys.iter().any(|recent_key| *recent_key == key)
    });
}

fn section_rank(section: &str) -> u8 {
    match section {
        "Favorites" => 0,
        "Recent" => 1,
        "Calculator" => 2,
        "Apps" => 4,
        "Projects" => 5,
        "Files" => 10,
        "Commands" => 20,
        "Web" => 30,
        _ => 100,
    }
}

fn can_favorite_result(result: &SearchResult) -> bool {
    result.default_action != "noop"
        && result.provider != "files"
        && result.provider != "file_search"
        && !(result.provider == "web_search" && result.id.contains(':'))
}

fn global_result_cmp(a: &SearchResult, b: &SearchResult) -> Ordering {
    b.score
        .total_cmp(&a.score)
        .then_with(|| a.provider.cmp(b.provider))
        .then_with(|| a.title.cmp(&b.title))
        .then_with(|| a.id.cmp(&b.id))
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
    use crate::provider::{
        ResultKind,
        SearchAction,
    };

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
            kind: ResultKind::App,
            section: "Apps".to_string(),
            title: id.to_string(),
            subtitle: String::new(),
            icon: String::new(),
            score,
            default_action: "open".to_string(),
            actions: vec![SearchAction::new("open", "Open", "")],
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
    fn sort_results_keeps_sections_contiguous() {
        let mut app_high = result("apps", "ghostty.desktop", 2.0);
        app_high.title = "Ghostty".to_string();

        let mut command = result("commands", "hamlet", 1.5);
        command.kind = ResultKind::Command;
        command.section = "Commands".to_string();
        command.title = "Hamlet".to_string();

        let mut app_low = result("apps", "dolphin.desktop", 1.0);
        app_low.title = "Dolphin".to_string();

        let mut results = vec![app_low, command, app_high];

        sort_results(&mut results);

        assert_eq!(results[0].id, "ghostty.desktop");
        assert_eq!(results[1].id, "dolphin.desktop");
        assert_eq!(results[2].id, "hamlet");
    }

    #[test]
    fn sort_results_uses_explicit_section_order() {
        let mut command = result("commands", "hamlet", 3.0);
        command.kind = ResultKind::Command;
        command.section = "Commands".to_string();
        command.title = "Hamlet".to_string();

        let mut file = result("files", "/tmp/notes.md", 2.5);
        file.kind = ResultKind::File;
        file.section = "Files".to_string();
        file.title = "notes.md".to_string();

        let mut project = result("projects", "/tmp/rika", 2.0);
        project.kind = ResultKind::Project;
        project.section = "Projects".to_string();
        project.title = "rika".to_string();

        let mut app = result("apps", "ghostty.desktop", 1.0);
        app.title = "Ghostty".to_string();

        let mut results = vec![command, app, project, file];

        sort_results(&mut results);

        assert_eq!(results[0].section, "Apps");
        assert_eq!(results[1].section, "Projects");
        assert_eq!(results[2].section, "Files");
        assert_eq!(results[3].section, "Commands");
    }

    #[test]
    fn recent_results_uses_last_used_order_and_recent_section() {
        let mut store = UsageStore::default();
        store.records.insert(
            result_usage_key("apps", "old.desktop"),
            UsageRecord {
                provider: "apps".to_string(),
                id: "old.desktop".to_string(),
                activation_count: 10,
                last_used_at_unix: 1,
            },
        );
        store.records.insert(
            result_usage_key("apps", "new.desktop"),
            UsageRecord {
                provider: "apps".to_string(),
                id: "new.desktop".to_string(),
                activation_count: 1,
                last_used_at_unix: 2,
            },
        );

        let results = vec![
            result("apps", "old.desktop", 1.0),
            result("apps", "new.desktop", 1.0),
        ];

        let recent = store.recent_results(&results, 1);

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "new.desktop");
        assert_eq!(recent[0].section, "Recent");
    }

    #[test]
    fn remove_synthetic_duplicates_removes_matching_base_results() {
        let mut recent = result("apps", "ghostty.desktop", 1.0);
        recent.section = "Recent".to_string();

        let mut results = vec![
            recent,
            result("apps", "ghostty.desktop", 1.0),
            result("apps", "kitty.desktop", 1.0),
        ];

        remove_synthetic_duplicates(&mut results);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].section, "Recent");
        assert_eq!(results[1].id, "kitty.desktop");
    }

    #[test]
    fn remove_synthetic_duplicates_prefers_favorites_over_recent() {
        let mut favorite = result("apps", "ghostty.desktop", 1.0);
        favorite.section = "Favorites".to_string();
        let mut recent = result("apps", "ghostty.desktop", 1.0);
        recent.section = "Recent".to_string();

        let mut results = vec![favorite, recent, result("apps", "ghostty.desktop", 1.0)];

        remove_synthetic_duplicates(&mut results);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].section, "Favorites");
    }

    #[test]
    fn favorite_results_use_favorites_section() {
        let mut store = UsageStore::default();
        assert!(store.handle_favorite_action("apps", "ghostty.desktop", ADD_FAVORITE_ACTION));

        let results = vec![
            result("apps", "ghostty.desktop", 1.0),
            result("apps", "kitty.desktop", 1.0),
        ];

        let favorites = store.favorite_results(&results);

        assert_eq!(favorites.len(), 1);
        assert_eq!(favorites[0].id, "ghostty.desktop");
        assert_eq!(favorites[0].section, "Favorites");
    }

    #[test]
    fn add_result_actions_reflects_favorite_state() {
        let mut store = UsageStore::default();
        assert!(store.handle_favorite_action("apps", "ghostty.desktop", ADD_FAVORITE_ACTION));

        let mut results = vec![
            result("apps", "ghostty.desktop", 1.0),
            result("apps", "kitty.desktop", 1.0),
        ];

        store.add_result_actions(&mut results);

        assert!(
            results[0]
                .actions
                .iter()
                .any(|action| action.id == REMOVE_FAVORITE_ACTION
                    && action.success_message == "Removed from Favorites")
        );
        assert!(
            results[1]
                .actions
                .iter()
                .any(|action| action.id == ADD_FAVORITE_ACTION
                    && action.success_message == "Added to Favorites")
        );
    }

    #[test]
    fn add_result_actions_skips_direct_file_results() {
        let store = UsageStore::default();
        let mut file = result("files", "/tmp/notes.md", 1.0);
        file.kind = ResultKind::File;

        let mut results = vec![file];

        store.add_result_actions(&mut results);

        assert!(
            !results[0]
                .actions
                .iter()
                .any(|action| action.id == ADD_FAVORITE_ACTION
                    || action.id == REMOVE_FAVORITE_ACTION)
        );
    }

    #[test]
    fn add_result_actions_skips_indexed_file_results() {
        let store = UsageStore::default();
        let mut file = result("file_search", "/tmp/notes.md", 1.0);
        file.kind = ResultKind::File;

        let mut results = vec![file];

        store.add_result_actions(&mut results);

        assert!(
            !results[0]
                .actions
                .iter()
                .any(|action| action.id == ADD_FAVORITE_ACTION
                    || action.id == REMOVE_FAVORITE_ACTION)
        );
    }

    #[test]
    fn favorite_results_use_newest_first_order() {
        let mut store = UsageStore::default();
        store.favorites.insert(
            result_usage_key("apps", "old.desktop"),
            FavoriteRecord {
                provider: "apps".to_string(),
                id: "old.desktop".to_string(),
                favorited_at_unix: 1,
            },
        );
        store.favorites.insert(
            result_usage_key("apps", "new.desktop"),
            FavoriteRecord {
                provider: "apps".to_string(),
                id: "new.desktop".to_string(),
                favorited_at_unix: 2,
            },
        );

        let favorites = store.favorite_results(&[
            result("apps", "old.desktop", 1.0),
            result("apps", "new.desktop", 1.0),
        ]);

        assert_eq!(favorites[0].id, "new.desktop");
        assert_eq!(favorites[1].id, "old.desktop");
    }

    #[test]
    fn save_and_load_round_trips_favorites() {
        let path = temp_state_path("favorites-round-trip");
        let mut store = UsageStore::load_from_path(path.clone());
        assert!(store.handle_favorite_action("apps", "ghostty.desktop", ADD_FAVORITE_ACTION));
        store.save().expect("usage state should save");

        let loaded = UsageStore::load_from_path(path.clone());

        assert!(
            loaded
                .favorites
                .contains_key(&result_usage_key("apps", "ghostty.desktop"))
        );

        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn load_defaults_legacy_favorite_timestamps() {
        let path = temp_state_path("legacy-favorite");
        fs::create_dir_all(path.parent().expect("state file should have parent"))
            .expect("temp dir should be created");
        fs::write(
            &path,
            r#"{"entries":[],"favorites":[{"provider":"apps","id":"ghostty.desktop"}]}"#,
        )
        .expect("legacy state should be written");

        let loaded = UsageStore::load_from_path(path.clone());
        let favorite = loaded
            .favorites
            .get(&result_usage_key("apps", "ghostty.desktop"))
            .expect("legacy favorite should load");

        assert_eq!(favorite.favorited_at_unix, 0);

        if let Some(parent) = path.parent() {
            let _ = fs::remove_dir_all(parent);
        }
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
