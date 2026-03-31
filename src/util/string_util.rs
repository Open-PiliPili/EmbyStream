pub struct StringUtil;

impl StringUtil {
    pub fn trim_trailing_slashes(input: &str) -> &str {
        input.trim_end_matches('/')
    }

    pub fn hash_hex(input: &str) -> String {
        if input.is_empty() {
            return "".to_string();
        }
        Self::hash_bytes(input.as_bytes())
    }

    pub fn hash_bytes(input: &[u8]) -> String {
        if input.is_empty() {
            return "".to_string();
        }
        blake3::hash(input).to_hex().to_string()
    }
}
