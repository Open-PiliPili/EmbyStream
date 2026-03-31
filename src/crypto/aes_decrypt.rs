use std::collections::HashMap;

use aes::Aes128;
use aes::cipher::{
    BlockDecryptMut, KeyIvInit, block_padding::Pkcs7,
    generic_array::GenericArray,
};
use base64::{
    Engine,
    engine::general_purpose::{
        STANDARD as BASE64, URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD,
    },
};
use cbc::Decryptor;

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

        Self::decrypt_msgpack_urlsafe(encrypted, key, iv).or_else(
            |msgpack_err| {
                warn_old_format_fallback(&msgpack_err);
                Self::decrypt_json_standard(encrypted, key, iv)
            },
        )
    }

    fn decrypt_msgpack_urlsafe(
        encrypted: &str,
        key: &str,
        iv: &str,
    ) -> Result<HashMap<String, String>, Error> {
        let decoded = BASE64_URL_SAFE_NO_PAD
            .decode(encrypted)
            .map_err(Error::Base64DecodeError)?;
        let decrypted = Self::decrypt_bytes(&decoded, key, iv)?;
        let dict: HashMap<String, String> =
            rmp_serde::from_slice(&decrypted)
                .map_err(|e| Error::DecryptionError(e.to_string()))?;

        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Decryption successful, restored MessagePack dictionary"
        );
        Ok(dict)
    }

    fn decrypt_json_standard(
        encrypted: &str,
        key: &str,
        iv: &str,
    ) -> Result<HashMap<String, String>, Error> {
        let decoded = BASE64.decode(encrypted).map_err(|e| {
            error_log!(
                CRYPTO_LOGGER_DOMAIN,
                "Failed to decode legacy Base64 string: {}",
                e
            );
            Error::Base64DecodeError(e)
        })?;
        let decrypted = Self::decrypt_bytes(&decoded, key, iv)?;
        let dict: HashMap<String, String> =
            serde_json::from_slice(&decrypted).map_err(Error::JsonError)?;

        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Decryption successful, restored legacy JSON dictionary"
        );
        Ok(dict)
    }

    fn decrypt_bytes(
        ciphertext: &[u8],
        key: &str,
        iv: &str,
    ) -> Result<Vec<u8>, Error> {
        let key = KeyNormalizer::normalize_from_str(key)?;
        let iv_bytes = KeyNormalizer::normalize_from_str(iv)?;
        let iv = GenericArray::from_slice(&iv_bytes);
        let cipher =
            Aes128CbcDecryptor::new(GenericArray::from_slice(&key), iv);

        let mut buffer = ciphertext.to_vec();
        let decrypted = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|e| {
                error_log!(CRYPTO_LOGGER_DOMAIN, "Decryption failed: {}", e);
                Error::DecryptionError(e.to_string())
            })?;

        Ok(decrypted.to_vec())
    }
}

fn warn_old_format_fallback(error: &Error) {
    debug_log!(
        CRYPTO_LOGGER_DOMAIN,
        "MessagePack/base64url decrypt failed, trying legacy JSON/base64 fallback: {}",
        error
    );
}
