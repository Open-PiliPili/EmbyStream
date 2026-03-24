//! Mask secrets in TOML text for `config show`.

const SENSITIVE_KEY_NAMES: &[&str] =
    &["token", "password", "encipher_key", "encipher_iv"];

fn key_is_sensitive(key: &str) -> bool {
    let k = key.trim().trim_matches('"').to_ascii_lowercase();
    SENSITIVE_KEY_NAMES.iter().any(|s| k == *s)
}

/// Mask lines that look like `key = "value"` for sensitive keys.
pub fn mask_toml_secrets(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    for line in content.lines() {
        let t = line.trim_start();
        let replaced = if let Some((key, _)) = t.split_once('=') {
            if key_is_sensitive(key) {
                format!("{}= \"***\"", key.trim_end())
            } else {
                line.to_string()
            }
        } else {
            line.to_string()
        };
        out.push_str(&replaced);
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redact_secret(s: &str) -> String {
        if s.len() <= 8 {
            "***".to_string()
        } else {
            format!("{}...{}", &s[..4], &s[s.len() - 4..])
        }
    }

    #[test]
    fn masks_token_line() {
        let t = r#"token = "secretvaluehere""#;
        let m = mask_toml_secrets(t);
        assert!(m.contains("***"));
        assert!(!m.contains("secretvaluehere"));
    }

    #[test]
    fn redact_short() {
        assert_eq!(redact_secret("ab"), "***");
    }

    #[test]
    fn redact_long() {
        let r = redact_secret("abcdefghijklmnop");
        assert!(r.starts_with("abcd"));
        assert!(r.ends_with("mnop"));
    }
}
