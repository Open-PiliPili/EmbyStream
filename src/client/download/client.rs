use std::collections::HashMap;
use crate::{
    api::download::API,
    client::BuildableClient,
    network::{NetworkPlugin, NetworkProvider},
};
use reqwest::Response;

pub struct Client {
    provider: NetworkProvider,
}

impl BuildableClient for Client {
    fn build_from_plugins(plugins: Vec<Box<dyn NetworkPlugin>>) -> Self {
        let provider = NetworkProvider::new(plugins);
        Client { provider }
    }
}

impl Client {
    pub async fn download(
        &self,
        remote_url: impl Into<String>,
        user_agent: impl Into<Option<String>>,
        forward_headers: Option<HashMap<String, String>>,
    ) -> Result<Response, anyhow::Error> {
        let request = API::download(
            remote_url.into(),
            user_agent.into(),
            forward_headers
        );
        let response = self.provider.send_request(&request).await?;
        Ok(response)
    }
}