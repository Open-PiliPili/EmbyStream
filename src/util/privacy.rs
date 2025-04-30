/// Privacy utilities for handling sensitive data.
pub struct Privacy;

impl Privacy {
    /// Creates a new Privacy instance.
    pub fn new() -> Self {
        Privacy
    }

    /// Desensitizes a sensitive string by showing the first 4 characters and masking the rest.
    ///
    /// # Arguments
    ///
    /// * `s` - The string to desensitize.
    ///
    /// # Returns
    ///
    /// A desensitized string. If empty, returns "<empty>". If length <= 4, returns the original string.
    /// Otherwise, returns the first 4 characters followed by "***".
    pub fn desensitize(&self, s: &str) -> String {
        if s.is_empty() {
            "<empty>".to_string()
        } else if s.len() <= 4 {
            s.to_string()
        } else {
            format!("{}***", &s[..4])
        }
    }
}
