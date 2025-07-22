use std::collections::HashMap;

use aes::Aes128;
use aes::cipher::{
    BlockDecryptMut, KeyIvInit, block_padding::Pkcs7,
    generic_array::GenericArray,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cbc::Decryptor;
use serde_json;

use super::key_normalizer::KeyNormalizer;
use crate::{CRYPTO_LOGGER_DOMAIN, Error, debug_log, error_log};

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
    /// * `key` - A string slice that will be used as the encryption key.
    ///   The key will be converted to bytes, padded with `0` if shorter than 16 bytes,
    ///   or truncated if longer than 16 bytes.
    /// * `iv` - An optional 16-byte initialization vector. If not provided, a default IV of `[0; 16]` is used.
    ///   when encoded to UTF-8, used for AES-128-CBC decryption.
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<String, String>)` - The decrypted dictionary.
    /// * `Err(Error)` - If the key length is invalid, Base64 decoding fails, or decryption fails.
    pub fn decrypt(
        encrypted: &str,
        key: &str,
        iv: &str,
    ) -> Result<HashMap<String, String>, Error> {
        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Starting AES decryption for Base64 string"
        );

        // Decode Base64
        let decoded = BASE64.decode(encrypted).map_err(|e| {
            error_log!(
                CRYPTO_LOGGER_DOMAIN,
                "Failed to decode Base64 string: {}",
                e
            );
            Error::Base64DecodeError(e)
        })?;

        // Validate key length
        let key = KeyNormalizer::normalize_from_str(key)?;

        // Validate iv length
        let iv_bytes = KeyNormalizer::normalize_from_str(iv)?;
        let iv = GenericArray::from_slice(&iv_bytes);

        let ciphertext = &decoded;

        // Initialize cipher
        let cipher =
            Aes128CbcDecryptor::new(GenericArray::from_slice(&key), iv);

        // Copy ciphertext to mutable buffer for in-place decryption
        let mut buffer = ciphertext.to_vec();
        let decrypted = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| {
                error_log!(CRYPTO_LOGGER_DOMAIN, "Decryption failed: {}", e);
                Error::DecryptionError(e.to_string())
            })?;

        // Deserialize JSON to dictionary
        let dict: HashMap<String, String> = serde_json::from_slice(decrypted)
            .map_err(|e| {
            error_log!(
                CRYPTO_LOGGER_DOMAIN,
                "Failed to deserialize JSON to dictionary: {}",
                e
            );
            Error::JsonError(e)
        })?;

        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Decryption successful, restored dictionary"
        );
        Ok(dict)
    }
}
