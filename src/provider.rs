pub trait Provider {
    fn id(&self) -> &'static str;

    fn search(&self, query: &str) -> Vec<SearchResult>;
}

pub struct SearchResult {
    pub title: String,
    pub provider: &'static str,
    pub action: Action,
}

pub enum Action {
    Echo(String),
}

pub struct MockProvider {
    pub entries: Vec<&'static str>,
}

impl MockProvider {
    pub fn new() -> Self {
        let entries = vec!["foo", "bar", "baz", "qux"];

        Self { entries }
    }
}

impl Provider for MockProvider {
    fn id(&self) -> &'static str {
        "mock"
    }

    fn search(&self, query: &str) -> Vec<SearchResult> {
        let mut results = vec![];

        for entry in &self.entries {
            if entry.contains(query) {
                results.push(SearchResult {
                    title: entry.to_string(),
                    provider: self.id(),
                    action: Action::Echo(format!("exec action for '{}'", entry)),
                });
            }
        }

        results
    }
}
