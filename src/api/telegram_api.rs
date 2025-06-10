use crate::{
    api::TelegramOperation,
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo,
};

use super::{PhotoMessage, TextMessage};

/// The base URL for the Telegram API, used to construct requests to the Telegram Bot API.
/// This constant provides the root address, to be concatenated with a bot token and specific endpoints.
const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";

/// Represents Telegram Bot API endpoints with their respective parameters.
#[derive(Debug, Clone)]
pub struct TelegramAPI {
    /// The bot token for authenticating with the Telegram API.
    token: String,
    /// The default chat ID for sending messages.
    chat_id: String,
    /// The specific API operation (SendMessage or SendPhoto).
    operation: TelegramOperation,
}

impl TelegramAPI {
    pub fn text(
        token: impl Into<String>,
        chat_id: impl Into<String>,
        params: TextMessage,
    ) -> Self {
        TelegramAPI {
            token: token.into(),
            chat_id: chat_id.into(),
            operation: TelegramOperation::SendMessage(params),
        }
    }

    pub fn photo(
        token: impl Into<String>,
        chat_id: impl Into<String>,
        params: PhotoMessage,
    ) -> Self {
        TelegramAPI {
            token: token.into(),
            chat_id: chat_id.into(),
            operation: TelegramOperation::SendPhoto(params),
        }
    }

    /// Gets the target chat ID.
    fn get_chat_id(&self) -> String {
        self.chat_id.clone()
    }
}

impl NetworkTarget for TelegramAPI {
    /// Gets the base URL for Telegram API requests.
    ///
    /// Constructs the URL using the bot token from configuration.
    fn base_url(&self) -> String {
        format!("{}{}", TELEGRAM_API_BASE, self.token)
    }

    /// Gets the API endpoint path for the specific operation.
    fn path(&self) -> String {
        match &self.operation {
            TelegramOperation::SendMessage(_) => "sendMessage".to_string(),
            TelegramOperation::SendPhoto(_) => "sendPhoto".to_string(),
        }
    }

    /// Gets the HTTP method for the request (always POST for Telegram API).
    fn method(&self) -> HttpMethod {
        HttpMethod::Post
    }

    /// Converts the API operation into a network task ready for execution.
    ///
    /// # Returns
    /// A `NetworkTask` containing all necessary request parameters.
    fn task(&self) -> NetworkTask {
        match &self.operation {
            TelegramOperation::SendMessage(params) => params.clone().into_task(self.get_chat_id()),
            TelegramOperation::SendPhoto(params) => params.clone().into_task(self.get_chat_id()),
        }
    }

    /// Gets the default headers for Telegram API requests.
    ///
    /// Includes:
    /// - Standard JSON content type headers
    /// - User agent string
    fn headers(&self) -> Option<Vec<(&'static str, String)>> {
        let sys_info = SystemInfo::new();
        Some(vec![
            ("Content-Type", "application/json".to_string()),
            ("Accept", "application/json".to_string()),
            ("user-agent", sys_info.get_user_agent()),
        ])
    }
}
