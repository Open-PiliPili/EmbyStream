use crate::{
    api::telegram::Operation,
    network::{HttpMethod, NetworkTarget, NetworkTask},
    system::SystemInfo,
};

use super::{PhotoMessage, TextMessage};

/// The base URL for the Telegram API, used to construct requests to the Telegram Bot API.
/// This constant provides the root address, to be concatenated with a bot token and specific endpoints.
const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";

/// Represents Telegram Bot API endpoints with their respective parameters.
#[derive(Debug, Clone)]
pub struct API {
    /// The bot token for authenticating with the Telegram API.
    token: String,
    /// The default chat ID for sending messages.
    chat_id: String,
    /// The specific API operation (SendMessage or SendPhoto).
    operation: Operation,
}

impl API {
    pub fn text(
        token: impl Into<String>,
        chat_id: impl Into<String>,
        params: TextMessage,
    ) -> Self {
        API {
            token: token.into(),
            chat_id: chat_id.into(),
            operation: Operation::SendMessage(params),
        }
    }

    pub fn photo(
        token: impl Into<String>,
        chat_id: impl Into<String>,
        params: PhotoMessage,
    ) -> Self {
        API {
            token: token.into(),
            chat_id: chat_id.into(),
            operation: Operation::SendPhoto(params),
        }
    }

    /// Gets the target chat ID.
    fn get_chat_id(&self) -> String {
        self.chat_id.clone()
    }
}

impl NetworkTarget for API {
    /// Gets the base URL for Telegram API requests.
    ///
    /// Constructs the URL using the bot token from configuration.
    fn base_url(&self) -> String {
        format!("{}{}", TELEGRAM_API_BASE, self.token)
    }

    /// Gets the API endpoint path for the specific operation.
    fn path(&self) -> String {
        match &self.operation {
            Operation::SendMessage(_) => "sendMessage".to_string(),
            Operation::SendPhoto(_) => "sendPhoto".to_string(),
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
            Operation::SendMessage(params) => {
                params.clone().into_task(self.get_chat_id())
            }
            Operation::SendPhoto(params) => {
                params.clone().into_task(self.get_chat_id())
            }
        }
    }

    /// Gets the default headers for Telegram API requests.
    ///
    /// Includes:
    /// - Standard JSON content type headers
    /// - User agent string
    fn headers(&self) -> Vec<(String, String)> {
        let sys_info = SystemInfo::new();
        vec![
            ("Content-Type".into(), "application/json".into()),
            ("Accept".into(), "application/json".into()),
            ("user-agent".into(), sys_info.get_user_agent()),
        ]
    }
}
