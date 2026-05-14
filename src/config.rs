use std::io::ErrorKind;

use anyhow::bail;
use serde::{
    Deserialize,
    Serialize,
};

use crate::providers::desktop_provider::DesktopProviderConfig;

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub launcher: Launcher,

    #[serde(default)]
    pub providers: Providers,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Launcher {
    pub max_visible_results: u8,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            max_visible_results: 7,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Providers {
    #[serde(default)]
    pub desktop: DesktopProviderConfig,
}

impl Config {
    pub fn load_config() -> anyhow::Result<Config> {
        let Some(mut config_path) = dirs::config_dir() else {
            bail!("failed to get user config dir")
        };

        config_path.push("rika");
        config_path.push("config.toml");

        let config_str = match std::fs::read_to_string(&config_path) {
            Ok(config_str) => config_str,
            Err(err) if err.kind() == ErrorKind::NotFound => {
                return Ok(Config::default());
            }
            Err(err) => bail!(
                "failed to read config file at {}: {err}",
                config_path.display()
            ),
        };

        let config = match toml::from_str::<Config>(&config_str) {
            Ok(config) => config,
            Err(err) => bail!(
                "failed to parse config file at {}: {err}",
                config_path.display()
            ),
        };

        Ok(config)
    }
}
