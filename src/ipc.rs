use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    config::Config,
    provider::SearchResult,
};

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    #[serde(rename = "query")]
    Query { request_id: u64, query: String },

    #[serde(rename = "activate")]
    Activate {
        provider: String,
        id: String,
        action: String,
    },

    #[serde(rename = "refresh")]
    Refresh { request_id: u64 },

    #[serde(rename = "config")]
    Config,
}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum ServerResponse {
    #[serde(rename = "results")]
    Results {
        request_id: u64,
        items: Vec<SearchResult>,
    },

    #[serde(rename = "activated")]
    Activated {
        provider: String,
        id: String,
        action: String,
    },

    #[serde(rename = "refreshed")]
    Refreshed {
        request_id: u64,
        config: Config,
        errors: Vec<String>,
    },

    #[serde(rename = "config")]
    Config { config: Config },

    #[serde(rename = "error")]
    Error { message: String },
}
