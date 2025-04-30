use crate::{Error, CRYPTO_LOGGER_DOMAIN, error_log};

pub struct KeyNormalizer;

impl KeyNormalizer {
    /// Normalize a key to 16 bytes:
    /// - If < 6 bytes → Error
    /// - If 6~15 bytes → pad with 0
    /// - If >16 bytes → truncate
    pub fn normalize(key: &[u8]) -> Result<[u8; 16], Error> {
        if key.len() < 6 {
            error_log!(
                CRYPTO_LOGGER_DOMAIN,
                "Encryption key must be at least 6 bytes, got {} bytes",
                key.len()
            );
            return Err(Error::InvalidEncipherKey(key.len()));
        }

        let mut key_16 = [0u8; 16];
        let copy_len = key.len().min(16);
        key_16[..copy_len].copy_from_slice(&key[..copy_len]);
        Ok(key_16)
    }

    pub fn normalize_from_str(key_str: &str) -> Result<[u8; 16], Error> {
        Self::normalize(key_str.as_bytes())
    }
}
