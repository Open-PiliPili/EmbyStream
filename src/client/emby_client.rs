use crate::{
    client::BuildableClient,
    network::{NetworkPlugin, NetworkProvider},
};

pub struct EmbyClient {
    #[allow(dead_code)]
    provider: NetworkProvider,
}

impl BuildableClient for EmbyClient {
    fn build_from_plugins(plugins: Vec<Box<dyn NetworkPlugin>>) -> Self {
        let provider = NetworkProvider::new(plugins);
        EmbyClient { provider }
    }
}
