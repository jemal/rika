use serde::{
    Deserialize,
    Serialize,
};

use crate::provider::SearchResult;

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
}
