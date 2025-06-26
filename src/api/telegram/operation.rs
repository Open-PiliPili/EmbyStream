use crate::{
    api::{PhotoMessage, TextMessage},
};

/// Represents Telegram Bot API endpoints with their respective parameters.
///
/// This enum encapsulates all supported Telegram API operations,
/// providing a type-safe way to construct API requests.
#[derive(Debug, Clone)]
pub enum Operation {
    /// Send a text message to a chat
    SendMessage(TextMessage),

    /// Send a photo to a chat
    SendPhoto(PhotoMessage),
}