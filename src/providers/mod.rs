use crate::{
    config::Config,
    provider::Provider,
    providers::{
        apps::AppsProvider,
        commands::CommandsProvider,
        files::FilesProvider,
        projects::ProjectsProvider,
        web_search::WebSearchProvider,
    },
};

pub mod apps;
pub mod commands;
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
