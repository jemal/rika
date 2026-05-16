use crate::{
    config::Config,
    provider::Provider,
    providers::{
        command_provider::CommandProvider,
        desktop_provider::DesktopProvider,
    },
};

pub mod command_provider;
pub mod desktop_provider;

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
        id: "desktop",
        enabled: |config| config.providers.desktop.enabled,
        build: |_| Box::new(DesktopProvider::new()),
        reload: ReloadMode::Preserve,
    },
    ProviderSpec {
        id: "commands",
        enabled: |config| config.providers.commands.enabled,
        build: |config| Box::new(CommandProvider::new(&config.providers.commands)),
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
