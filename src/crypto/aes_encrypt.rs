use std::collections::HashMap;

use serde_json;
use aes::Aes128;
use cbc::Encryptor;
use aes::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7, generic_array::GenericArray};
use base64::{
    Engine,
    engine::general_purpose::STANDARD as BASE64
};

use super::KeyNormalizer;
use crate::{CRYPTO_LOGGER_DOMAIN, Error, error_log, info_log};

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
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - The Base64-encoded encrypted string (IV + ciphertext).
    /// * `Err(Error)` - If the key length is invalid or encryption fails.
    pub fn encrypt(dict: &HashMap<String, String>, key: &str) -> Result<String, Error> {
        info_log!(CRYPTO_LOGGER_DOMAIN, "Starting AES encryption for dictionary");

        // Validate key length
        let key = KeyNormalizer::normalize_from_str(key)?;

        // Serialize dictionary to JSON
        let json = serde_json::to_string(dict).map_err(|e| {
            error_log!(CRYPTO_LOGGER_DOMAIN, "Failed to serialize dictionary to JSON: {}", e);
            Error::JsonError(e)
        })?;

        // Use reversed key as IV
        let mut reversed_key = key.to_vec();
        reversed_key.reverse();
        let iv = GenericArray::from_slice(&reversed_key);

        // Initialize cipher
        let cipher = Aes128CbcEncryptor::new(&GenericArray::from_slice(&key), &iv);

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

        // Prepend IV to ciphertext
        let mut result = Vec::new();
        result.extend_from_slice(iv.as_slice());
        result.extend_from_slice(ciphertext);

        // Encode to Base64
        let encoded = BASE64.encode(&result);
        info_log!(CRYPTO_LOGGER_DOMAIN, "Encryption successful, produced Base64 string");
        Ok(encoded)
    }
}
