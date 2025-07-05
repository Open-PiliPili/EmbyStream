use crate::{
    api::emby::{API, User, PlaybackInfo},
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
    /// Retrieves user information from the Emby server asynchronously.
    ///
    /// Constructs and sends an Emby API request using the provided base URL, API key, and user ID.
    ///
    /// # Arguments
    /// - `base_url`: The base URL of the Emby server (e.g., "https://api.emby.example.com").
    /// - `api_key`: The API key for authenticating with the Emby server.
    /// - `user_id`: The ID of the user to retrieve.
    ///
    /// # Returns
    /// A `Result` containing the Emby API response with the user data on success,
    /// or an error if the request fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The network request fails (e.g., connection issues).
    /// - The Emby API returns an error response (e.g., invalid API key or user ID).
    /// - The response JSON parsing fails.
    pub async fn get_user(
        &self,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<User, anyhow::Error> {
        let request = API::get_user(base_url, api_key, user_id);
        let response = self.provider.send_request(&request).await?;
        let result: User = response.json().await?;
        Ok(result)
    }

    /// Retrieves playback information for a media item from the Emby server asynchronously.
    ///
    /// Constructs and sends an Emby API request using the provided base URL, API key,
    /// item ID, and media source ID.
    ///
    /// # Arguments
    /// - `base_url`: The base URL of the Emby server (e.g., "https://api.emby.example.com").
    /// - `api_key`: The API key for authenticating with the Emby server.
    /// - `item_id`: The ID of the media item.
    /// - `media_source_id`: The ID of the media source for playback.
    ///
    /// # Returns
    /// A `Result` containing the Emby API response with the playback information on success,
    /// or an error if the request fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The network request fails (e.g., connection issues).
    /// - The Emby API returns an error response (e.g., invalid API key or item ID).
    /// - The response JSON parsing fails.
    pub async fn playback_info(
        &self,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        item_id: impl Into<String>,
        media_source_id: impl Into<String>,
    ) -> Result<PlaybackInfo, anyhow::Error> {
        let request = API::playback_info(base_url, api_key, item_id, media_source_id);
        let response = self.provider.send_request(&request).await?;
        let result: PlaybackInfo = response.json().await?;
        Ok(result)
    }
}