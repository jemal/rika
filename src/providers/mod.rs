use crate::{
    config::Config,
    provider::Provider,
    providers::desktop_provider::DesktopProvider,
};

pub mod desktop_provider;

/// Build the initial providers.
pub fn build(config: &Config) -> Vec<Box<dyn Provider>> {
    let mut providers: Vec<Box<dyn Provider>> = vec![];
    update(&mut providers, config);

    providers
}

/// Prune and add new providers, if any.
pub fn update(providers: &mut Vec<Box<dyn Provider>>, config: &Config) {
    ensure_provider(
        providers,
        "desktop",
        config.providers.desktop.enabled,
        || Box::new(DesktopProvider::new()),
    );
}

fn ensure_provider(
    providers: &mut Vec<Box<dyn Provider>>,
    id: &'static str,
    enabled: bool,
    build: impl FnOnce() -> Box<dyn Provider>,
) {
    let exists = providers.iter().any(|provider| provider.id() == id);

    match (enabled, exists) {
        (true, false) => providers.push(build()),
        (false, true) => providers.retain(|provider| provider.id() != id),
        _ => {}
    }
}
