use crate::{
    api::openlist::{API, FileResponse, LinkData, LinkResponse},
    client::BuildableClient,
    network::{NetworkPlugin, NetworkProvider},
};

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
    pub async fn fetch_file_path(
        &self,
        url: impl Into<String>,
        token: impl Into<String>,
        emby_path: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Result<String, anyhow::Error> {
        let emby_path = emby_path.into();
        if emby_path.is_empty() {
            return Ok("".to_string());
        }

        let request = API::fs_get(url, token, emby_path, user_agent);
        let response = self.provider.send_request(&request).await?;
        let json: FileResponse = response.json().await?;
        Ok(json.get_data().get_raw_url().unwrap_or_default())
    }

    pub async fn fetch_file_link(
        &self,
        url: impl Into<String>,
        token: impl Into<String>,
        emby_path: impl Into<String>,
        user_agent: impl Into<String>,
    ) -> Result<LinkData, anyhow::Error> {
        let emby_path = emby_path.into();
        if emby_path.is_empty() {
            return Ok(LinkData::default());
        }

        let request = API::fs_link(url, token, emby_path, user_agent);
        let response = self.provider.send_request(&request).await?;
        let json: LinkResponse = response.json().await?;
        Ok(json.get_data())
    }
}
