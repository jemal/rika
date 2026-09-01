use std::io::ErrorKind;

use anyhow::{
    Context,
    bail,
};
use serde::{
    Deserialize,
    Serialize,
};

const KANAGAWA_THEME_TOML: &str = include_str!("../resources/themes/kanagawa.toml");

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
    #[serde(skip_serializing)]
    pub theme: ThemeSelection,
    #[serde(skip_deserializing)]
    pub themes: LauncherThemes,
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
            theme: ThemeSelection::default(),
            themes: LauncherThemes::default(),
            window: LauncherWindow::default(),
        }
    }
}

/// A user's choice of theme: either one named theme used for both light and
/// dark contexts, or a named theme per desktop appearance for people who
/// want the launcher to follow the system light/dark setting.
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeSelection {
    Named(String),
    Split { dark: String, light: String },
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self::Named("kanagawa".to_string())
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

/// Loads a named theme, checking `~/.config/rika/themes/<name>.toml` before
/// falling back to the themes bundled with rika.
fn load_named_theme(name: &str) -> anyhow::Result<LauncherTheme> {
    if let Some(mut theme_path) = dirs::config_dir() {
        theme_path.push("rika");
        theme_path.push("themes");
        theme_path.push(format!("{name}.toml"));

        match std::fs::read_to_string(&theme_path) {
            Ok(theme_str) => {
                return toml::from_str(&theme_str).with_context(|| {
                    format!("while parsing theme file at {}", theme_path.display())
                });
            }
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => bail!(
                "failed to read theme file at {}: {err}",
                theme_path.display()
            ),
        }
    }

    let builtin_theme_toml = match name {
        "kanagawa" => KANAGAWA_THEME_TOML,
        other => bail!(
            "unknown theme \"{other}\": no built-in theme by that name and no \
             ~/.config/rika/themes/{other}.toml file"
        ),
    };

    toml::from_str(builtin_theme_toml)
        .with_context(|| format!("while parsing built-in theme \"{name}\""))
}

/// Resolves a `ThemeSelection` into a concrete dark/light pair, loading one
/// named theme (used for both) or two (one per desktop appearance).
fn resolve_theme_selection(selection: &ThemeSelection) -> anyhow::Result<LauncherThemes> {
    match selection {
        ThemeSelection::Named(name) => {
            let theme = load_named_theme(name)
                .with_context(|| format!("while loading theme \"{name}\""))?;
            Ok(LauncherThemes {
                dark: theme.clone(),
                light: theme,
            })
        }
        ThemeSelection::Split { dark, light } => Ok(LauncherThemes {
            dark: load_named_theme(dark)
                .with_context(|| format!("while loading dark theme \"{dark}\""))?,
            light: load_named_theme(light)
                .with_context(|| format!("while loading light theme \"{light}\""))?,
        }),
    }
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

        let mut config = match serde_json::from_str::<Config>(&config_str) {
            Ok(config) => config,
            Err(err) => bail!(
                "failed to parse config file at {}: {err}",
                config_path.display()
            ),
        };

        config.launcher.themes = resolve_theme_selection(&config.launcher.theme)
            .context("while resolving launcher theme")?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_kanagawa_theme_loads_and_matches_defaults() {
        let theme = load_named_theme("kanagawa").expect("kanagawa theme should load");

        assert_eq!(theme.primary, LauncherTheme::default().primary);
    }

    #[test]
    fn unknown_theme_name_is_rejected() {
        let err = load_named_theme("does-not-exist")
            .err()
            .expect("unknown theme should fail to load");

        assert!(err.to_string().contains("unknown theme"));
    }

    #[test]
    fn example_theme_files_parse_as_flat_palettes() {
        for entry in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/resources/themes"))
            .expect("resources/themes should exist")
        {
            let path = entry.expect("dir entry should be readable").path();
            let theme_str = std::fs::read_to_string(&path).expect("theme file should be readable");

            toml::from_str::<LauncherTheme>(&theme_str).unwrap_or_else(|err| {
                panic!(
                    "{} should parse as a flat LauncherTheme: {err}",
                    path.display()
                )
            });
        }
    }

    #[test]
    fn theme_selection_deserializes_from_a_bare_string() {
        let selection: ThemeSelection =
            serde_json::from_str(r#""kanagawa""#).expect("bare string should deserialize");

        assert!(matches!(selection, ThemeSelection::Named(name) if name == "kanagawa"));
    }

    #[test]
    fn theme_selection_deserializes_from_a_dark_light_object() {
        let selection: ThemeSelection = serde_json::from_str(r#"{"dark": "lua", "light": "sol"}"#)
            .expect("dark/light object should deserialize");

        assert!(matches!(
            selection,
            ThemeSelection::Split { dark, light } if dark == "lua" && light == "sol"
        ));
    }

    #[test]
    fn resolve_theme_selection_named_uses_one_theme_for_both() {
        let themes = resolve_theme_selection(&ThemeSelection::Named("kanagawa".to_string()))
            .expect("named theme should resolve");

        assert_eq!(themes.dark.primary, themes.light.primary);
        assert_eq!(themes.dark.primary, "#7e9cd8");
    }

    #[test]
    fn resolve_theme_selection_split_loads_each_name() {
        let themes = resolve_theme_selection(&ThemeSelection::Split {
            dark: "kanagawa".to_string(),
            light: "kanagawa".to_string(),
        })
        .expect("split theme should resolve");

        assert_eq!(themes.dark.primary, "#7e9cd8");
        assert_eq!(themes.light.primary, "#7e9cd8");
    }

    #[test]
    fn resolve_theme_selection_rejects_unknown_names() {
        let err = resolve_theme_selection(&ThemeSelection::Named("does-not-exist".to_string()))
            .err()
            .expect("unknown named theme should fail to resolve");
        assert!(err.to_string().contains("while loading theme"));

        let err = resolve_theme_selection(&ThemeSelection::Split {
            dark: "does-not-exist".to_string(),
            light: "kanagawa".to_string(),
        })
        .err()
        .expect("unknown split theme should fail to resolve");
        assert!(err.to_string().contains("while loading dark theme"));
    }

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
        assert!(matches!(
            config.launcher.theme,
            ThemeSelection::Named(ref name) if name == "kanagawa"
        ));
        assert_eq!(config.providers.files.open_command, "xdg-open");
        assert_eq!(config.providers.projects.roots, vec!["~/dev/projects"]);
        assert_eq!(config.providers.projects.default_action, "open_terminal");
        assert_eq!(config.providers.projects.actions.len(), 2);
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
