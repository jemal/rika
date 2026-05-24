use crate::{
    config::Config,
    provider::Provider,
    providers::{
        apps::AppsProvider,
        commands::CommandsProvider,
        file_search::FileSearchProvider,
        files::FilesProvider,
        projects::ProjectsProvider,
        web_search::WebSearchProvider,
    },
};

pub mod apps;
pub mod commands;
pub mod file_search;
pub mod files;
pub mod projects;
pub mod web_search;

struct ProviderSpec {
    id: &'static str,
    enabled: fn(&Config) -> bool,
    build: fn(&Config) -> Box<dyn Provider>,
    rebuild_on_config_reload: bool,
}

const PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        id: "apps",
        enabled: |config| config.providers.apps.enabled,
        build: |config| Box::new(AppsProvider::new(&config.providers.apps)),
        rebuild_on_config_reload: true,
    },
    ProviderSpec {
        id: "commands",
        enabled: |config| config.providers.commands.enabled,
        build: |config| Box::new(CommandsProvider::new(&config.providers.commands)),
        rebuild_on_config_reload: true,
    },
    ProviderSpec {
        id: "file_search",
        enabled: |config| {
            config.providers.file_search.enabled && !config.providers.file_search.roots.is_empty()
        },
        build: |config| {
            Box::new(FileSearchProvider::new(
                &config.providers.file_search,
                &config.providers.files,
            ))
        },
        rebuild_on_config_reload: true,
    },
    ProviderSpec {
        id: "files",
        enabled: |config| config.providers.files.enabled,
        build: |config| Box::new(FilesProvider::new(&config.providers.files)),
        rebuild_on_config_reload: true,
    },
    ProviderSpec {
        id: "projects",
        enabled: |config| config.providers.projects.enabled,
        build: |config| Box::new(ProjectsProvider::new(&config.providers.projects)),
        rebuild_on_config_reload: true,
    },
    ProviderSpec {
        id: "web_search",
        enabled: |config| config.providers.web_search.enabled,
        build: |config| Box::new(WebSearchProvider::new(&config.providers.web_search)),
        rebuild_on_config_reload: true,
    },
];

/// Build the initial providers.
pub fn build(config: &Config) -> Vec<Box<dyn Provider>> {
    let mut providers: Vec<Box<dyn Provider>> = vec![];
    update(&mut providers, config);

    providers
}

/// Apply provider config by removing, adding, or rebuilding providers.
pub fn update(providers: &mut Vec<Box<dyn Provider>>, config: &Config) -> Vec<&'static str> {
    let mut rebuilt = vec![];

    for spec in PROVIDERS {
        let index = providers
            .iter()
            .position(|provider| provider.id() == spec.id);

        match (spec.enabled)(config) {
            false => {
                if let Some(index) = index {
                    providers.remove(index);
                }
            }
            true => match index {
                None => {
                    providers.push((spec.build)(config));
                    rebuilt.push(spec.id);
                }
                Some(index) if spec.rebuild_on_config_reload => {
                    providers[index] = (spec.build)(config);
                    rebuilt.push(spec.id);
                }
                Some(_) => {}
            },
        }
    }

    rebuilt
}

#[cfg(test)]
mod tests {
    use std::time::{
        SystemTime,
        UNIX_EPOCH,
    };

    use super::*;

    fn missing_root(test_name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("rika-provider-{test_name}-{nanos}"))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn file_search_disabled_by_default() {
        let providers = build(&Config::default());

        assert!(
            !providers
                .iter()
                .any(|provider| provider.id() == "file_search")
        );
    }

    #[test]
    fn file_search_empty_roots_do_not_build_provider() {
        let mut config = Config::default();
        config.providers.file_search.enabled = true;

        let providers = build(&config);

        assert!(
            !providers
                .iter()
                .any(|provider| provider.id() == "file_search")
        );
    }

    #[test]
    fn file_search_skips_missing_roots_without_failing_construction() {
        let mut config = Config::default();
        config.providers.file_search.enabled = true;
        config.providers.file_search.roots = vec![missing_root("missing")];

        let providers = build(&config);
        let provider = providers
            .iter()
            .find(|provider| provider.id() == "file_search")
            .expect("file_search provider should be constructed");

        assert!(provider.search("notes").is_empty());
    }
}
