use crate::{
    config::Config,
    provider::Provider,
    providers::{
        apps::AppsProvider,
        commands::CommandsProvider,
    },
};

pub mod apps;
pub mod commands;

struct ProviderSpec {
    id: &'static str,
    enabled: fn(&Config) -> bool,
    build: fn(&Config) -> Box<dyn Provider>,
    reload: ReloadMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReloadMode {
    Preserve,
    Rebuild,
}

const PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        id: "apps",
        enabled: |config| config.providers.apps.enabled,
        build: |_| Box::new(AppsProvider::new()),
        reload: ReloadMode::Preserve,
    },
    ProviderSpec {
        id: "commands",
        enabled: |config| config.providers.commands.enabled,
        build: |config| Box::new(CommandsProvider::new(&config.providers.commands)),
        reload: ReloadMode::Rebuild,
    },
];

/// Build the initial providers.
pub fn build(config: &Config) -> Vec<Box<dyn Provider>> {
    let mut providers: Vec<Box<dyn Provider>> = vec![];
    update(&mut providers, config);

    providers
}

/// Apply provider config by removing, adding, or rebuilding providers.
pub fn update(providers: &mut Vec<Box<dyn Provider>>, config: &Config) {
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
                None => providers.push((spec.build)(config)),
                Some(index) if spec.reload == ReloadMode::Rebuild => {
                    providers[index] = (spec.build)(config);
                }
                Some(_) => {}
            },
        }
    }
}
