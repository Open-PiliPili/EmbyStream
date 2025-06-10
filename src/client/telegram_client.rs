use crate::{
    api::{TextMessage, PhotoMessage, TelegramAPI, TelegramResponse, MessageResult},
    client::BuildableClient,
    network::{NetworkPlugin, NetworkProvider}
};

pub struct TelegramClient {
    provider: NetworkProvider,
}

impl BuildableClient for TelegramClient {
    fn build_from_plugins(plugins: Vec<Box<dyn NetworkPlugin>>) -> Self {
        let provider = NetworkProvider::new(plugins);
        TelegramClient { provider }
    }
}

impl TelegramClient {
    /// Sends a text message to a Telegram chat asynchronously.
    ///
    /// Constructs and sends a Telegram API request using the provided token, chat ID,
    /// and text message configuration.
    ///
    /// # Arguments
    /// - `token`: The Telegram bot token for authentication.
    /// - `chat_id`: The ID of the target chat or channel.
    /// - `text`: The text message configuration, including content and optional settings.
    ///
    /// # Returns
    /// A `Result` containing the Telegram API response with the message result on success,
    /// or an error if the request fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The network request fails (e.g., connection issues).
    /// - The Telegram API returns an error response (e.g., invalid token or chat ID).
    /// - The response JSON parsing fails.
    pub async fn send_text(
        &self,
        token: impl Into<String>,
        chat_id: impl Into<String>,
        text: TextMessage,
    ) -> Result<TelegramResponse<MessageResult>, anyhow::Error> {
        let request = TelegramAPI::text(token, chat_id, text);
        let response = self.provider.send_request(&request).await?;
        let result: TelegramResponse<MessageResult> = response.json().await?;
        Ok(result)
    }

    /// Sends a photo message to a Telegram chat asynchronously.
    ///
    /// Constructs and sends a Telegram API request to upload and send a photo
    /// using the provided token, chat ID, and photo message configuration.
    ///
    /// # Arguments
    /// - `token`: The Telegram bot token for authentication.
    /// - `chat_id`: The ID of the target chat or channel.
    /// - `photo`: The photo message configuration, including image data or URL and optional settings.
    ///
    /// # Returns
    /// A `Result` containing the Telegram API response with the message result on success,
    /// or an error if the request fails.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The network request fails (e.g., connection issues).
    /// - The photo upload fails (e.g., invalid file format or size limit exceeded).
    /// - The Telegram API returns an error response (e.g., invalid token or chat ID).
    /// - The response JSON parsing fails.
    pub async fn send_photo(
        &self,
        token: impl Into<String>,
        chat_id: impl Into<String>,
        photo: PhotoMessage,
    ) -> Result<TelegramResponse<MessageResult>, anyhow::Error> {
        let request = TelegramAPI::photo(token, chat_id, photo);
        let response = self.provider.send_request(&request).await?;
        let result: TelegramResponse<MessageResult> = response.json().await?;
        Ok(result)
    }
}
