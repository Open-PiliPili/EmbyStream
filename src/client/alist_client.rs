use crate::{
    AlistLinkResponse,
    api::{AlistAPI, AlistFileResponse, AlistLinkData},
    client::BuildableClient,
    network::{NetworkPlugin, NetworkProvider},
};

pub struct AlistClient {
    provider: NetworkProvider,
}

impl BuildableClient for AlistClient {
    fn build_from_plugins(plugins: Vec<Box<dyn NetworkPlugin>>) -> Self {
        let provider = NetworkProvider::new(plugins);
        AlistClient { provider }
    }
}

impl AlistClient {
    pub async fn fetch_file_path(
        &self,
        url: impl Into<String>,
        token: impl Into<String>,
        emby_path: impl Into<String>,
    ) -> Result<String, anyhow::Error> {
        let emby_path = emby_path.into();
        if emby_path.is_empty() {
            return Ok("".to_string());
        }

        let request = AlistAPI::fs_get(url, token, emby_path);
        let response = self.provider.send_request(&request).await?;
        let json: AlistFileResponse = response.json().await?;
        Ok(json.get_data().get_raw_url().unwrap_or_default())
    }

    pub async fn fetch_file_link(
        &self,
        url: impl Into<String>,
        token: impl Into<String>,
        emby_path: impl Into<String>,
    ) -> Result<AlistLinkData, anyhow::Error> {
        let emby_path = emby_path.into();
        if emby_path.is_empty() {
            return Ok(AlistLinkData::default());
        }

        let request = AlistAPI::fs_link(url, token, emby_path);
        let response = self.provider.send_request(&request).await?;
        let json: AlistLinkResponse = response.json().await?;
        Ok(json.get_data())
    }
}
