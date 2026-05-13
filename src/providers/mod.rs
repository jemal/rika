use crate::{
    provider::Provider,
    providers::desktop_provider::DesktopProvider,
};

pub mod desktop_provider;

pub fn build() -> Vec<Box<dyn Provider>> {
    let providers: Vec<Box<dyn Provider>> = vec![Box::new(DesktopProvider::new())];

    providers
}
