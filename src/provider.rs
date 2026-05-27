use serde::Serialize;

pub trait Provider: Send {
    fn id(&self) -> &'static str;
    fn search(&self, query: &str) -> Vec<SearchResult>;
    fn activate(&self, id: &str, action: &str) -> anyhow::Result<()>;
    fn refresh(&mut self) -> anyhow::Result<()>;
}

#[derive(Clone, Serialize)]
pub struct SearchResult {
    pub id: String,
    pub provider: &'static str,
    pub kind: ResultKind,
    pub section: String,
    pub title: String,
    pub subtitle: String,
    pub icon: String,
    pub score: f32,
    pub default_action: String,
    pub actions: Vec<SearchAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultKind {
    App,
    Calculator,
    Command,
    Directory,
    File,
    Project,
    Web,
}

#[derive(Clone, Serialize)]
pub struct SearchAction {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub close_behavior: SearchActionCloseBehavior,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub success_message: String,
}

impl SearchAction {
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            close_behavior: SearchActionCloseBehavior::Confirmed,
            success_message: String::new(),
        }
    }

    pub fn immediate(mut self) -> Self {
        self.close_behavior = SearchActionCloseBehavior::Immediate;
        self
    }

    pub fn keep_open(mut self) -> Self {
        self.close_behavior = SearchActionCloseBehavior::KeepOpen;
        self
    }

    pub fn success_message(mut self, message: impl Into<String>) -> Self {
        self.success_message = message.into();
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchActionCloseBehavior {
    Confirmed,
    Immediate,
    KeepOpen,
}
