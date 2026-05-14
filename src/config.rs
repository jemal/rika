use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub launcher: Launcher,
}

#[derive(Serialize, Deserialize)]
pub struct Launcher {
    pub max_visible_results: u8,
}
