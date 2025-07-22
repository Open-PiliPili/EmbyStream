use std::fmt;

/// Builder for creating Telegram MarkdownV2 formatted text.
///
/// Provides a fluent interface for constructing properly escaped MarkdownV2 content
/// that complies with Telegram's formatting requirements.
#[derive(Debug, Default)]
pub struct MarkdownV2Builder {
    /// The internal text buffer holding the MarkdownV2 content.
    /// This field stores the string as it is built, allowing incremental modifications.
    text: String,
}

impl MarkdownV2Builder {
    /// Creates a new empty MarkdownV2 builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends plain text with automatic escaping of special characters.
    pub fn text(mut self, text: &str) -> Self {
        self.text.push_str(&Self::escape(text));
        self
    }

    /// Appends bold-formatted text (`*bold*`).
    pub fn bold(self, text: &str) -> Self {
        self.text(&format!("*{}*", Self::escape(text)))
    }

    /// Appends italic-formatted text (`_italic_`).
    pub fn italic(self, text: &str) -> Self {
        self.text(&format!("_{}_", Self::escape(text)))
    }

    /// Appends an inline link (`[text](url)`).
    pub fn link(self, text: &str, url: &str) -> Self {
        self.text(&format!("[{}]({})", Self::escape(text), Self::escape(url)))
    }

    /// Finalizes and returns the built MarkdownV2 string.
    pub fn build(self) -> String {
        self.text
    }

    /// Escapes special MarkdownV2 characters in text.
    ///
    /// Telegram requires escaping these characters when they appear in regular text:
    /// `_ * [ ] ( ) ~ ` > # + - = | { } . !`
    fn escape(text: &str) -> String {
        const CHARS_TO_ESCAPE: &[char] = &[
            '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=',
            '|', '{', '}', '.', '!',
        ];

        text.chars().fold(String::new(), |mut s, c| {
            if CHARS_TO_ESCAPE.contains(&c) {
                s.push('\\');
            }
            s.push(c);
            s
        })
    }
}

impl fmt::Display for MarkdownV2Builder {
    /// Formats the Markdown content for display.
    ///
    /// Note: This shows the raw Markdown text, not the rendered version.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}
