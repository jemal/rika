use serde::Serialize;

pub trait Provider: Send {
    fn id(&self) -> &'static str;
    fn search(&self, query: &str) -> Vec<SearchResult>;
    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()>;
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
