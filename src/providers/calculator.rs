use std::{
    fs,
    path::PathBuf,
    sync::Mutex,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use anyhow::{
    Context,
    bail,
};
use evalexpr::{
    Value,
    eval,
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

const HISTORY_LIMIT: usize = 25;
const HISTORY_SECTION: &str = "Calculator History";

pub struct CalculatorProvider {
    history: Mutex<CalculatorHistory>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CalculatorProviderConfig {
    pub enabled: bool,
}

impl Default for CalculatorProviderConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl CalculatorProvider {
    pub fn new(_config: &CalculatorProviderConfig) -> Self {
        Self {
            history: Mutex::new(CalculatorHistory::load()),
        }
    }

    #[cfg(test)]
    fn with_history(history: CalculatorHistory) -> Self {
        Self {
            history: Mutex::new(history),
        }
    }
}

impl Provider for CalculatorProvider {
    fn id(&self) -> &'static str {
        "calculator"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let query = query.trim();

        if query == "=" {
            return self
                .history
                .lock()
                .expect("calculator history mutex should not be poisoned")
                .results(self.id());
        }

        let Some(expression) = expression_from_query(query) else {
            return vec![];
        };

        let Some(result) = calculate(expression, query.starts_with('=')) else {
            return vec![];
        };

        vec![calculation_result(
            self.id(),
            expression,
            result,
            "Calculator",
            1.0,
            false,
        )]
    }

    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()> {
        match action {
            "copy_result" => {
                let Some(result) = calculate(id, true) else {
                    bail!("invalid calculator expression: {id}");
                };

                clipboard::copy_text(&result)
                    .context("while attempting to copy calculator result")?;

                if let Err(err) = self
                    .history
                    .lock()
                    .expect("calculator history mutex should not be poisoned")
                    .record(id, &result)
                {
                    eprintln!("failed to record calculator history: {err}");
                }

                Ok(())
            }
            "copy_expression" => {
                clipboard::copy_text(id).context("while attempting to copy calculator expression")
            }
            "remove_history" => self
                .history
                .lock()
                .expect("calculator history mutex should not be poisoned")
                .remove(id)
                .context("while attempting to remove calculator history"),
            _ => bail!("unsupported calculator action: {action}"),
        }
    }

    fn refresh(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Deserialize, Serialize)]
struct CalculatorHistoryEntry {
    expression: String,
    result: String,
    last_used_at_unix: u64,
}

#[derive(Default, Deserialize, Serialize)]
struct CalculatorHistoryState {
    #[serde(default)]
    entries: Vec<CalculatorHistoryEntry>,
}

#[derive(Default)]
struct CalculatorHistory {
    path: Option<PathBuf>,
    entries: Vec<CalculatorHistoryEntry>,
}

impl CalculatorHistory {
    fn load() -> Self {
        let Some(path) = state_file_path() else {
            eprintln!("failed to resolve state directory for calculator history");
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
                    entries: vec![],
                };
            }
            Err(err) => {
                eprintln!("failed to read {}: {err}", path.display());
                return Self {
                    path: Some(path),
                    entries: vec![],
                };
            }
        };

        let state = match serde_json::from_str::<CalculatorHistoryState>(&content) {
            Ok(state) => state,
            Err(err) => {
                eprintln!("failed to parse {}: {err}", path.display());
                return Self {
                    path: Some(path),
                    entries: vec![],
                };
            }
        };

        let mut history = Self {
            path: Some(path),
            entries: state.entries,
        };
        history.normalize();
        history
    }

    fn results(&self, provider: &'static str) -> Vec<SearchResult> {
        self.entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                calculation_result(
                    provider,
                    &entry.expression,
                    entry.result.clone(),
                    HISTORY_SECTION,
                    1.0 - (index as f32 * 0.001),
                    true,
                )
            })
            .collect()
    }

    fn record(&mut self, expression: &str, result: &str) -> anyhow::Result<()> {
        self.entries
            .retain(|entry| entry.expression.as_str() != expression);
        self.entries.insert(
            0,
            CalculatorHistoryEntry {
                expression: expression.to_string(),
                result: result.to_string(),
                last_used_at_unix: current_unix_timestamp(),
            },
        );
        self.entries.truncate(HISTORY_LIMIT);
        self.save()
    }

    fn remove(&mut self, expression: &str) -> anyhow::Result<()> {
        self.entries
            .retain(|entry| entry.expression.as_str() != expression);
        self.save()
    }

    fn normalize(&mut self) {
        self.entries
            .sort_by(|a, b| b.last_used_at_unix.cmp(&a.last_used_at_unix));
        self.entries.dedup_by(|a, b| a.expression == b.expression);
        self.entries.truncate(HISTORY_LIMIT);
    }

    fn save(&self) -> anyhow::Result<()> {
        let Some(path) = &self.path else {
            return Ok(());
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("while attempting to create calculator state directory")?;
        }

        let content = serde_json::to_string_pretty(&CalculatorHistoryState {
            entries: self.entries.clone(),
        })
        .context("while attempting to serialize calculator history")?;
        let tmp_path = path.with_extension("json.tmp");
        fs::write(&tmp_path, content).context("while attempting to write calculator history")?;
        fs::rename(&tmp_path, path).context("while attempting to replace calculator history")?;

        Ok(())
    }
}

fn calculation_result(
    provider: &'static str,
    expression: &str,
    result: String,
    section: &str,
    score: f32,
    history_entry: bool,
) -> SearchResult {
    let mut actions = vec![
        SearchAction::new("copy_result", "Copy Result", "").immediate(),
        SearchAction::new("copy_expression", "Copy Expression", "").immediate(),
    ];

    if history_entry {
        actions.push(SearchAction::new("remove_history", "Remove from History", "").keep_open());
    }

    SearchResult {
        id: expression.to_string(),
        provider,
        kind: ResultKind::Calculator,
        section: section.to_string(),
        title: result,
        subtitle: expression.to_string(),
        icon: "builtin:calculator".to_string(),
        score,
        default_action: "copy_result".to_string(),
        actions,
        autocomplete: Some(format!("= {expression}")),
    }
}

fn expression_from_query(query: &str) -> Option<&str> {
    let query = query.trim();
    let expression = query.strip_prefix('=').map(str::trim).unwrap_or(query);

    if expression.is_empty() {
        None
    } else {
        Some(expression)
    }
}

fn calculate(query: &str, forced: bool) -> Option<String> {
    let query = query.trim();
    if !looks_like_calculation(query, forced) {
        return None;
    }

    let value = eval(query).ok()?;
    match value {
        Value::Int(value) => Some(value.to_string()),
        Value::Float(value) if value.is_finite() => Some(format_float(value)),
        _ => None,
    }
}

fn looks_like_calculation(query: &str, forced: bool) -> bool {
    if query.is_empty() || !query.chars().any(|c| c.is_ascii_digit()) {
        return false;
    }

    if forced {
        return true;
    }

    query
        .chars()
        .any(|c| matches!(c, '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')'))
}

fn format_float(value: f64) -> String {
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }

    value.to_string()
}

fn state_file_path() -> Option<PathBuf> {
    Some(
        dirs::state_dir()?
            .join("rika")
            .join("calculator_history.json"),
    )
}

fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_basic_arithmetic() {
        let provider = CalculatorProvider::new(&CalculatorProviderConfig { enabled: true });

        let results = provider.search("2 + 2 * 3");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].kind, ResultKind::Calculator);
        assert_eq!(results[0].section, "Calculator");
        assert_eq!(results[0].title, "8");
        assert_eq!(results[0].subtitle, "2 + 2 * 3");
        assert!(
            results[0]
                .actions
                .iter()
                .any(|action| action.id == results[0].default_action)
        );
    }

    #[test]
    fn ignores_non_numeric_queries() {
        let provider = CalculatorProvider::new(&CalculatorProviderConfig { enabled: true });

        assert!(provider.search("firefox").is_empty());
        assert!(provider.search("1").is_empty());
        assert!(provider.search("1 == 1").is_empty());
    }

    #[test]
    fn equal_prefix_forces_calculator_mode() {
        let provider = CalculatorProvider::new(&CalculatorProviderConfig { enabled: true });

        let results = provider.search("=1");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "1");
        assert_eq!(results[0].subtitle, "1");
        assert_eq!(results[0].autocomplete.as_deref(), Some("= 1"));
    }

    #[test]
    fn equal_query_returns_history() {
        let provider = CalculatorProvider::with_history(CalculatorHistory {
            path: None,
            entries: vec![CalculatorHistoryEntry {
                expression: "2 + 2".to_string(),
                result: "4".to_string(),
                last_used_at_unix: 10,
            }],
        });

        let results = provider.search("=");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].section, HISTORY_SECTION);
        assert_eq!(results[0].title, "4");
        assert_eq!(results[0].subtitle, "2 + 2");
        assert_eq!(results[0].autocomplete.as_deref(), Some("= 2 + 2"));
        assert!(
            results[0]
                .actions
                .iter()
                .any(|action| action.id == "copy_expression")
        );
        assert!(
            results[0]
                .actions
                .iter()
                .any(|action| action.id == "remove_history")
        );
    }

    #[test]
    fn history_records_newest_first_and_deduplicates() {
        let mut history = CalculatorHistory::default();

        history.record("2 + 2", "4").expect("history is in-memory");
        history.record("3 + 3", "6").expect("history is in-memory");
        history.record("2 + 2", "4").expect("history is in-memory");

        assert_eq!(history.entries.len(), 2);
        assert_eq!(history.entries[0].expression, "2 + 2");
        assert_eq!(history.entries[1].expression, "3 + 3");
    }
}
