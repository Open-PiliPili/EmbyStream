use std::collections::HashMap;

use serde_json;
use aes::Aes128;
use cbc::Decryptor;
use aes::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7, generic_array::GenericArray};
use base64::{
    Engine,
    engine::general_purpose::STANDARD as BASE64
};

use super::KeyNormalizer;
use crate::{CRYPTO_LOGGER_DOMAIN, Error, error_log, info_log};

// Create type alias for AES-128-CBC Decryptor
type Aes128CbcDecryptor = Decryptor<Aes128>;

/// AES decryption utility for decrypting a Base64-encoded string into a dictionary.
pub struct AesDecrypt;

impl AesDecrypt {
    /// Decrypts a Base64-encoded string into a dictionary using AES-128-CBC.
    ///
    /// # Arguments
    ///
    /// * `encrypted` - The Base64-encoded string (IV + ciphertext).
    /// * `key` - The decryption key as a UTF-8 string, must be at least 6 characters long.
    ///           The key will be converted to bytes, padded with `0` if shorter than 16 bytes,
    ///           or truncated if longer than 16 bytes.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, String>)` - The decrypted dictionary.
    /// * `Err(Error)` - If the key length is invalid, Base64 decoding fails, or decryption fails.
    pub fn decrypt(encrypted: &str, key: &str) -> Result<HashMap<String, String>, Error> {
        info_log!(CRYPTO_LOGGER_DOMAIN, "Starting AES decryption for Base64 string");

        // Validate key length
        let key = KeyNormalizer::normalize_from_str(key)?;

        // Decode Base64
        let decoded = BASE64.decode(encrypted).map_err(|e| {
            error_log!(CRYPTO_LOGGER_DOMAIN, "Failed to decode Base64 string: {}", e);
            Error::Base64DecodeError(e)
        })?;

        // Extract IV (first 16 bytes) and ciphertext
        if decoded.len() < 16 {
            error_log!(CRYPTO_LOGGER_DOMAIN, "Decoded data too short to contain IV");
            return Err(Error::DecryptionError(
                "Decoded data too short to contain IV".to_string(),
            ));
        }
        let iv = GenericArray::from_slice(&decoded[0..16]);
        let ciphertext = &decoded[16..];

        // Initialize cipher
        let cipher = Aes128CbcDecryptor::new(&GenericArray::from_slice(&key), iv);

        // Copy ciphertext to mutable buffer for in-place decryption
        let mut buffer = ciphertext.to_vec();
        let decrypted = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| {
                error_log!(CRYPTO_LOGGER_DOMAIN, "Decryption failed: {}", e);
                Error::DecryptionError(e.to_string())
            })?;

        // Deserialize JSON to dictionary
        let dict: HashMap<String, String> = serde_json::from_slice(decrypted).map_err(|e| {
            error_log!(CRYPTO_LOGGER_DOMAIN, "Failed to deserialize JSON to dictionary: {}", e);
            Error::JsonError(e)
        })?;

        info_log!(CRYPTO_LOGGER_DOMAIN, "Decryption successful, restored dictionary");
        Ok(dict)
    }
}
