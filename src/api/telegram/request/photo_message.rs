use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    path::PathBuf,
};

use serde::Serialize;

use crate::network::NetworkTask;

/// Represents the input source for a photo message.
///
/// This enum supports both remote URLs and local file paths as photo sources,
/// providing flexibility in how photos are supplied to the Telegram API.
#[derive(Debug, Clone)]
pub enum PhotoInput {
    /// A photo from a remote URL
    Url(String),

    /// A photo from a local file path
    FilePath(PathBuf),
}

impl Display for PhotoInput {
    /// Formats the photo input for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            PhotoInput::Url(url) => write!(f, "[URL] {}", url),
            PhotoInput::FilePath(path) => {
                write!(f, "[File] {}", path.display())
            }
        }
    }
}

/// Represents a photo message to be sent via Telegram API.
///
/// Contains the photo source and an optional caption with MarkdownV2 formatting support.
#[derive(Debug, Clone, Serialize)]
pub struct PhotoMessage {
    /// The photo source (local file or URL)
    #[serde(skip_serializing)]
    pub photo: PhotoInput,

    /// Optional caption for the photo with MarkdownV2 formatting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
}

impl PhotoMessage {
    /// Converts the photo message into a network task for sending.
    ///
    /// # Arguments
    /// * `chat_id` - The target chat ID for the message
    ///
    /// # Returns
    /// A `NetworkTask` ready for execution by the network infrastructure.
    ///
    /// # Notes
    /// - For file paths, creates a multipart request with file upload
    /// - For URLs, creates a standard multipart request
    /// - Automatically sets parse mode to MarkdownV2
    pub fn into_task(self, chat_id: String) -> NetworkTask {
        let mut fields = HashMap::new();
        fields.insert("chat_id".to_string(), chat_id);
        fields.insert("parse_mode".to_string(), "MarkdownV2".to_string());

        if let Some(caption) = self.caption {
            fields.insert("caption".to_string(), caption);
        }

        match self.photo {
            PhotoInput::FilePath(path) => {
                let files = vec![(
                    path.to_string_lossy().into_owned(),
                    "photo".to_string(),
                )];
                NetworkTask::RequestMultipartWithFiles(fields, files)
            }
            PhotoInput::Url(url) => {
                fields.insert("photo".to_string(), url);
                NetworkTask::RequestMultipart(fields)
            }
        }
    }

    /// Creates a new photo message from a file path.
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        Self {
            photo: PhotoInput::FilePath(path.into()),
            caption: None,
        }
    }

    /// Creates a new photo message from a URL.
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            photo: PhotoInput::Url(url.into()),
            caption: None,
        }
    }

    /// Sets the caption for the photo message.
    pub fn with_caption(mut self, caption: impl Into<String>) -> Self {
        self.caption = Some(caption.into());
        self
    }
}

impl Display for PhotoMessage {
    /// Formats the photo message for display purposes.
    ///
    /// Shows the photo source and optional caption if present.
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "PhotoMessage(photo: {}", self.photo)?;
        if let Some(caption) = &self.caption {
            write!(f, ", caption: {}", caption)?;
        }
        write!(f, ")")
    }
}
