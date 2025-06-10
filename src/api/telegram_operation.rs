use crate::{
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo,
};

use super::{PhotoMessage, TextMessage};

/// The base URL for the Telegram API, used to construct requests to the Telegram Bot API.
/// This constant provides the root address, to be concatenated with a bot token and specific endpoints.
const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";

/// Represents Telegram Bot API endpoints with their respective parameters.
///
/// This enum encapsulates all supported Telegram API operations,
/// providing a type-safe way to construct API requests.
#[derive(Debug, Clone)]
pub enum TelegramOperation {
    /// Send a text message to a chat
    SendMessage(TextMessage),

    /// Send a photo to a chat
    SendPhoto(PhotoMessage),
}

impl NetworkTarget for TelegramOperation {
    /// Gets the base URL for Telegram API requests.
    ///
    /// Constructs the URL using the bot token from configuration.
    fn base_url(&self) -> String {
        let token = "".to_string();
        format!("{}{}", TELEGRAM_API_BASE, token)
    }

    /// Gets the API endpoint path for the specific operation.
    fn path(&self) -> String {
        match self {
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
        match self {
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

impl TelegramOperation {
    /// Gets the target chat ID from configuration.
    ///
    /// This is used as the default destination for all messages.
    fn get_chat_id(&self) -> String {
        "".to_string()
    }
}
