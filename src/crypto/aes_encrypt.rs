use std::collections::HashMap;

use aes::Aes128;
use aes::cipher::{
    BlockEncryptMut, KeyIvInit, block_padding::Pkcs7,
    generic_array::GenericArray,
};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use cbc::Encryptor;
use serde_json;

use super::key_normalizer::KeyNormalizer;
use crate::{CRYPTO_LOGGER_DOMAIN, Error, debug_log, error_log};

// Create type alias for AES-128-CBC Encryptor
type Aes128CbcEncryptor = Encryptor<Aes128>;

/// AES encryption utility for encrypting a dictionary.
pub struct AesEncrypt;

impl AesEncrypt {
    /// Encrypts a dictionary into a Base64-encoded string using AES-128-CBC.
    ///
    /// # Arguments
    ///
    /// * `dict` - The dictionary to encrypt (HashMap<String, String>).
    /// * `key` - The decryption key as a UTF-8 string, must be at least 6 characters long.
    ///           The key will be converted to bytes, padded with `0` if shorter than 16 bytes,
    ///           or truncated if longer than 16 bytes.
    /// * `iv` - The initialization vector as a UTF-8 string, must be at least 6 characters long.
    ///          when encoded to UTF-8, used for AES-128-CBC decryption.
    /// # Returns
    ///
    /// * `Ok(String)` - The Base64-encoded encrypted string (IV + ciphertext).
    /// * `Err(Error)` - If the key length is invalid or encryption fails.
    pub fn encrypt(
        dict: &HashMap<String, String>,
        key: &str,
        iv: &str,
    ) -> Result<String, Error> {
        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Starting AES encryption for dictionary"
        );

        // Serialize dictionary to JSON
        let json = serde_json::to_string(dict).map_err(|e| {
            error_log!(
                CRYPTO_LOGGER_DOMAIN,
                "Failed to serialize dictionary to JSON: {}",
                e
            );
            Error::JsonError(e)
        })?;

        // Validate key length
        let key = KeyNormalizer::normalize_from_str(key)?;

        // Validate iv length
        let iv_bytes = KeyNormalizer::normalize_from_str(iv)?;
        let iv = GenericArray::from_slice(&iv_bytes);

        // Initialize cipher
        let cipher =
            Aes128CbcEncryptor::new(GenericArray::from_slice(&key), iv);

        // Encrypt the JSON string with PKCS7 padding
        let plaintext = json.as_bytes();
        // Allocate output buffer: input length + one block (16 bytes) for padding
        let mut output = vec![0u8; plaintext.len() + 16];
        let ciphertext = cipher
            .encrypt_padded_b2b_mut::<Pkcs7>(plaintext, &mut output)
            .map_err(|e| {
                error_log!(CRYPTO_LOGGER_DOMAIN, "Encryption failed: {}", e);
                Error::EncryptionError(e.to_string())
            })?;

        // Encode to Base64
        let encoded = BASE64.encode(ciphertext);
        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Encryption successful, produced Base64 string"
        );
        Ok(encoded)
    }
}
