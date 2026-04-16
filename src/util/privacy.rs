use form_urlencoded::Serializer;

/// Privacy utilities for handling sensitive data.
pub struct Privacy;

impl Default for Privacy {
    fn default() -> Self {
        Self::new()
    }
}

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
    /// A desensitized string. If empty, returns "<empty>".
    /// If length <= 4, returns the original string.
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

    pub fn mask_google_drive_token(raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return "<empty>".to_string();
        }

        let (scheme, token) = trimmed
            .split_once(' ')
            .map(|(prefix, value)| (format!("{prefix} "), value))
            .unwrap_or_else(|| (String::new(), trimmed));

        let chars: Vec<char> = token.chars().collect();
        if chars.is_empty() {
            return format!("{scheme}<empty>");
        }
        if chars.len() <= 8 {
            return format!("{scheme}{}", "*".repeat(chars.len()));
        }

        let prefix: String = chars.iter().take(4).copied().collect();
        let suffix: String = chars
            .iter()
            .rev()
            .take(4)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{scheme}{prefix}...{suffix}")
    }

    pub fn sanitize_google_drive_internal_path_for_log(path: &str) -> String {
        let Some((base, query)) = path.split_once('?') else {
            return path.to_string();
        };

        let sanitized_query = form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .map(|(key, value)| {
                if key == "token" {
                    (key, Self::mask_google_drive_token(value.as_str()))
                } else {
                    (key, value)
                }
            })
            .fold(
                Serializer::new(String::new()),
                |mut serializer, (key, value)| {
                    serializer.append_pair(&key, &value);
                    serializer
                },
            )
            .finish();

        format!("{base}?{sanitized_query}")
    }
}

#[cfg(test)]
mod tests {
    use super::Privacy;

    #[test]
    fn mask_google_drive_token_keeps_prefix_and_suffix_only() {
        assert_eq!(
            Privacy::mask_google_drive_token("Bearer access-token"),
            "Bearer acce...oken"
        );
        assert_eq!(Privacy::mask_google_drive_token("short"), "*****");
        assert_eq!(Privacy::mask_google_drive_token(""), "<empty>");
    }

    #[test]
    fn sanitize_google_drive_internal_path_masks_token_query() {
        let path = "/_origin/google-drive/gd-node/file%2Did%2D123?\
token=Bearer+access-token&foo=bar";

        assert_eq!(
            Privacy::sanitize_google_drive_internal_path_for_log(path),
            "/_origin/google-drive/gd-node/file%2Did%2D123?\
token=Bearer+acce...oken&foo=bar"
        );
    }
}
