use serde::Serialize;

pub trait Provider {
    fn id(&self) -> &'static str;
    fn search(&self, query: &str) -> Vec<SearchResult>;
}

#[derive(Serialize)]
pub struct SearchResult {
    pub id: String,
    pub provider: &'static str,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f32,
    pub actions: Vec<String>,
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
                    id: format!("mock:{entry}"),
                    provider: self.id(),
                    title: entry.to_string(),
                    subtitle: entry.to_string(),
                    icon: "!".to_string(),
                    score: 1.0,
                    actions: vec!["open".to_string(), "open-terminal".to_string()],
                });
            }
        }

        results
    }
}
