use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::Deserialize;

/// Represents a response from the Telegram Bot API.
///
/// This is a generic wrapper for all Telegram API responses that follows their standard format.
/// The actual response data is contained in the `result` field when successful.
#[derive(Debug, Deserialize)]
pub struct TelegramResponse<T> {
    /// Indicates whether the request was successful
    pub ok: bool,

    /// The actual response data if the request was successful
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,

    /// Human-readable description of the error if the request failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl<T: Display> Display for TelegramResponse<T> {
    /// Formats the Telegram response for display purposes.
    ///
    /// Shows the status, error description (if any), and result (if any).
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Status: {}", if self.ok { "OK" } else { "Failed" })?;

        if let Some(desc) = &self.description {
            write!(f, ", Description: {}", desc)?;
        }

        if let Some(res) = &self.result {
            write!(f, ", Result: {}", res)?;
        }

        Ok(())
    }
}

/// Represents a successful message sent via Telegram API.
///
/// Contains metadata about the sent message including its ID and destination chat.
#[derive(Debug, Deserialize)]
pub struct MessageResult {
    /// Unique message identifier
    pub message_id: i64,

    /// The chat this message was sent to
    pub chat: Chat,

    /// The actual text content of the message (if it was a text message)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl Display for MessageResult {
    /// Formats the message result for display purposes.
    ///
    /// Shows the message ID, chat info, and text content (if available).
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "MessageID: {}, Chat: {}", self.message_id, self.chat)?;

        if let Some(text) = &self.text {
            write!(f, ", Text: {}", text)?;
        }

        Ok(())
    }
}

/// Represents a Telegram chat or channel.
///
/// This could be a private chat, group, supergroup, or channel.
#[derive(Debug, Deserialize)]
pub struct Chat {
    /// Unique identifier for this chat
    pub id: i64,

    /// First name of the other party in a private chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Username of the other party in a private chat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    /// Type of chat (private, group, supergroup, or channel)
    #[serde(rename = "type")]
    pub chat_type: String,
}

impl Display for Chat {
    /// Formats the chat info for display purposes.
    ///
    /// Shows the chat ID, type, and available identifying information.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "ID: {}, Type: {}", self.id, self.chat_type)?;

        if let Some(name) = &self.first_name {
            write!(f, ", Name: {}", name)?;
        }

        if let Some(username) = &self.username {
            write!(f, ", Username: @{}", username)?;
        }

        Ok(())
    }
}

impl Chat {
    /// Returns a display-friendly name for the chat.
    ///
    /// Uses the first available of:
    /// 1. First name + username (if both exist)
    /// 2. First name
    /// 3. Username
    /// 4. Chat type + ID
    pub fn display_name(&self) -> String {
        match (&self.first_name, &self.username) {
            (Some(name), Some(user)) => format!("{} (@{})", name, user),
            (Some(name), None) => name.clone(),
            (None, Some(user)) => format!("@{}", user),
            (None, None) => format!("{} {}", self.chat_type, self.id),
        }
    }
}
