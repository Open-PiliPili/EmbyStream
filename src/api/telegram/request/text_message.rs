use std::fmt::{Display, Formatter, Result as FmtResult};

use serde::Serialize;
use serde_json::Value;

use crate::network::NetworkTask;

/// Represents a text message to be sent via Telegram API.
///
/// Supports MarkdownV2 formatting and optional reply markup for interactive keyboards.
#[derive(Debug, Clone, Serialize)]
pub struct TextMessage {
    /// The message text content with MarkdownV2 formatting support
    pub text: String,

    /// Optional inline keyboard or reply markup in JSON string format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<String>,
}

impl TextMessage {
    /// Creates a new text message with the given content.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            reply_markup: None,
        }
    }

    /// Sets the reply markup (inline keyboard) for the message.
    pub fn with_reply_markup(mut self, markup: impl Into<String>) -> Self {
        self.reply_markup = Some(markup.into());
        self
    }

    /// Converts the message to a JSON value with required Telegram API fields.
    ///
    /// Automatically adds:
    /// - `parse_mode: "MarkdownV2"`
    /// - `chat_id` from parameter
    pub fn to_json_value(&self, chat_id: String) -> Value {
        let mut value = serde_json::to_value(self)
            .expect("Failed to serialize TextMessage");

        if let Some(obj) = value.as_object_mut() {
            obj.entry("parse_mode")
                .or_insert_with(|| "MarkdownV2".into());
            obj.entry("chat_id").or_insert_with(|| chat_id.into());
        }

        value
    }

    /// Converts the message into a network task ready for sending.
    ///
    /// Wraps the JSON payload in a `NetworkTask::RequestJson` variant.
    pub fn into_task(self, chat_id: String) -> NetworkTask {
        NetworkTask::RequestJson(self.to_json_value(chat_id))
    }
}

impl Display for TextMessage {
    /// Formats the message for display, showing only the text content.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "text={}", self.text)
    }
}
