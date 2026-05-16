use std::io::ErrorKind;

use anyhow::bail;
use serde::{
    Deserialize,
    Serialize,
};

use crate::providers::{
    command_provider::CommandProviderConfig,
    desktop_provider::DesktopProviderConfig,
};

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub launcher: Launcher,
    pub providers: Providers,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Launcher {
    pub max_visible_results: u8,
    pub font_family: String,
    pub font_size: u8,
    pub small_font_size: u8,
    pub tiny_font_size: u8,
    pub window: LauncherWindow,
}

impl Default for Launcher {
    fn default() -> Self {
        Self {
            max_visible_results: 7,
            font_family: String::new(),
            font_size: 14,
            small_font_size: 13,
            tiny_font_size: 10,
            window: LauncherWindow::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherWindow {
    pub anchor: LauncherWindowAnchor,
    pub width: u16,
    pub height: u16,
    pub margin: u16,
}

impl Default for LauncherWindow {
    fn default() -> Self {
        Self {
            anchor: LauncherWindowAnchor::Top,
            width: 580,
            height: 316,
            margin: 320,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherWindowAnchor {
    #[default]
    Top,
    Center,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Providers {
    pub desktop: DesktopProviderConfig,
    pub commands: CommandProviderConfig,
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
