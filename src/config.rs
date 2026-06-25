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
    pub color_scheme: LauncherColorScheme,
    pub themes: LauncherThemes,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<LauncherTheme>,
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
            color_scheme: LauncherColorScheme::Auto,
            themes: LauncherThemes::default(),
            theme: None,
            window: LauncherWindow::default(),
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LauncherColorScheme {
    #[default]
    Auto,
    Light,
    Dark,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherThemes {
    pub light: LauncherTheme,
    pub dark: LauncherTheme,
}

impl Default for LauncherThemes {
    fn default() -> Self {
        Self {
            light: LauncherTheme::light(),
            dark: LauncherTheme::dark(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LauncherTheme {
    pub dim: String,
    pub dim_opacity: f32,
    pub surface: String,
    pub surface_variant: String,
    pub hover: String,
    pub outline: String,
    pub outline_opacity: f32,
    pub primary: String,
    pub accent: String,
    pub warning: String,
    pub error: String,
    pub text: String,
    pub muted_text: String,
}

impl Default for LauncherTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl LauncherTheme {
    fn dark() -> Self {
        Self {
            dim: "#0d0c0c".to_string(),
            dim_opacity: 0.22,
            surface: "#181820".to_string(),
            surface_variant: "#1f1f28".to_string(),
            hover: "#2a2a37".to_string(),
            outline: "#2a2a37".to_string(),
            outline_opacity: 0.46,
            primary: "#7e9cd8".to_string(),
            accent: "#98bb6c".to_string(),
            warning: "#ff9e3b".to_string(),
            error: "#e82424".to_string(),
            text: "#dcd7ba".to_string(),
            muted_text: "#727169".to_string(),
        }
    }

    fn light() -> Self {
        Self {
            dim: "#1f1f28".to_string(),
            dim_opacity: 0.14,
            surface: "#dcd5ac".to_string(),
            surface_variant: "#f2ecbc".to_string(),
            hover: "#b5cbd2".to_string(),
            outline: "#716e61".to_string(),
            outline_opacity: 0.36,
            primary: "#4d699b".to_string(),
            accent: "#6f894e".to_string(),
            warning: "#cc6d00".to_string(),
            error: "#e82424".to_string(),
            text: "#545464".to_string(),
            muted_text: "#8a8980".to_string(),
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
        assert!(matches!(
            config.launcher.color_scheme,
            LauncherColorScheme::Auto
        ));
        assert_eq!(config.launcher.themes.dark.primary, "#7e9cd8");
        assert_eq!(config.launcher.themes.dark.text, "#dcd7ba");
        assert_eq!(config.launcher.themes.dark.surface_variant, "#1f1f28");
        assert_eq!(config.launcher.themes.dark.warning, "#ff9e3b");
        assert_eq!(config.launcher.themes.dark.error, "#e82424");
        assert_eq!(config.launcher.themes.dark.dim_opacity, 0.22);
        assert_eq!(config.launcher.themes.dark.outline_opacity, 0.46);
        assert_eq!(config.launcher.themes.light.primary, "#4d699b");
        assert_eq!(config.launcher.themes.light.surface, "#dcd5ac");
        assert_eq!(config.launcher.themes.light.hover, "#b5cbd2");
        assert_eq!(config.launcher.themes.light.outline, "#716e61");
        assert_eq!(config.launcher.themes.light.outline_opacity, 0.36);
        assert_eq!(config.launcher.themes.light.warning, "#cc6d00");
        assert!(config.launcher.theme.is_none());
        assert_eq!(config.providers.files.open_command, "xdg-open");
        assert_eq!(config.providers.projects.roots, vec!["~/dev/projects"]);
        assert_eq!(config.providers.projects.default_action, "open_terminal");
        assert_eq!(config.providers.projects.actions.len(), 2);
    }

    #[test]
    fn legacy_single_launcher_theme_deserializes() {
        let config = serde_json::from_str::<Config>(
            r##"
{
  "launcher": {
    "theme": {
      "surface": "#ffffff",
      "text": "#111111",
      "primary": "#0055aa"
    }
  }
}
"##,
        )
        .expect("legacy theme config should deserialize");

        let theme = config.launcher.theme.expect("legacy theme should be kept");
        assert_eq!(theme.surface, "#ffffff");
        assert_eq!(theme.text, "#111111");
        assert_eq!(theme.primary, "#0055aa");
        assert_eq!(theme.accent, "#98bb6c");
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
