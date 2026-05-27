use std::io::ErrorKind;

use anyhow::bail;
use serde::{
    Deserialize,
    Serialize,
};

use crate::providers::{
    apps::AppsProviderConfig,
    calculator::CalculatorProviderConfig,
    commands::CommandsProviderConfig,
    file_search::FileSearchProviderConfig,
    files::FilesProviderConfig,
    projects::ProjectsProviderConfig,
    web_search::WebSearchProviderConfig,
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
    pub apps: AppsProviderConfig,
    pub calculator: CalculatorProviderConfig,
    pub commands: CommandsProviderConfig,
    pub file_search: FileSearchProviderConfig,
    pub files: FilesProviderConfig,
    pub projects: ProjectsProviderConfig,
    pub web_search: WebSearchProviderConfig,
}

impl Config {
    pub fn load_config() -> anyhow::Result<Config> {
        let Some(mut config_path) = dirs::config_dir() else {
            bail!("failed to get user config dir")
        };

        config_path.push("rika");
        config_path.push("config.json");

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

        let config = match serde_json::from_str::<Config>(&config_str) {
            Ok(config) => config,
            Err(err) => bail!(
                "failed to parse config file at {}: {err}",
                config_path.display()
            ),
        };

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_config_deserializes() {
        let config = serde_json::from_str::<Config>(include_str!("../resources/config.json"))
            .expect("packaged config should deserialize");

        assert!(config.providers.projects.enabled);
        assert!(config.providers.calculator.enabled);
        assert!(config.providers.files.enabled);
        assert!(!config.providers.file_search.enabled);
        assert!(config.providers.file_search.roots.is_empty());
        assert_eq!(config.providers.file_search.min_query_len, 3);
        assert_eq!(config.providers.file_search.max_results, 50);
        assert_eq!(config.providers.files.open_command, "xdg-open");
        assert_eq!(config.providers.projects.roots, vec!["~/dev/projects"]);
        assert_eq!(config.providers.projects.kitty_command, "kitty");
        assert_eq!(config.providers.projects.kitty_remote, "auto");
    }

    #[test]
    fn unknown_provider_fields_are_rejected() {
        let err = serde_json::from_str::<Config>(
            r#"
{
  "providers": {
    "file_search": {
      "enabled": false,
      "unknown": true
    }
  }
}
"#,
        )
        .err()
        .expect("unknown provider fields should fail");

        assert!(err.to_string().contains("unknown"));
    }
}
