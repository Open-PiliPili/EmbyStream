use std::collections::HashMap;

use aes::Aes128;
use aes::cipher::{
    BlockEncryptMut, KeyIvInit, block_padding::Pkcs7,
    generic_array::GenericArray,
};
use base64::{
    Engine, engine::general_purpose::URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD,
};
use cbc::Encryptor;

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
    /// * `key` - A string slice that will be used as the encryption key.
    ///   The key will be converted to bytes, padded with `0` if shorter than 16 bytes,
    ///   or truncated if longer than 16 bytes.
    /// * `iv` - An optional 16-byte initialization vector. If not provided, a default IV of `[0; 16]` is used.
    ///   when encoded to UTF-8, used for AES-128-CBC decryption.
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

        // Serialize dictionary to MessagePack for shorter deterministic payloads.
        let payload = rmp_serde::to_vec_named(dict).map_err(|e| {
            error_log!(
                CRYPTO_LOGGER_DOMAIN,
                "Failed to serialize dictionary to MessagePack: {}",
                e
            );
            Error::EncryptionError(e.to_string())
        })?;

        // Validate key length
        let key = KeyNormalizer::normalize_from_str(key)?;

        // Validate iv length
        let iv_bytes = KeyNormalizer::normalize_from_str(iv)?;
        let iv = GenericArray::from_slice(&iv_bytes);

        // Initialize cipher
        let cipher =
            Aes128CbcEncryptor::new(GenericArray::from_slice(&key), iv);

        // Encrypt the MessagePack payload with PKCS7 padding
        let plaintext = payload.as_slice();
        // Allocate output buffer: input length + one block (16 bytes) for padding
        let mut output = vec![0u8; plaintext.len() + 16];
        let ciphertext = cipher
            .encrypt_padded_b2b_mut::<Pkcs7>(plaintext, &mut output)
            .map_err(|e| {
                error_log!(CRYPTO_LOGGER_DOMAIN, "Encryption failed: {}", e);
                Error::EncryptionError(e.to_string())
            })?;

        // Encode to URL-safe Base64 without padding to shorten query usage.
        let encoded = BASE64_URL_SAFE_NO_PAD.encode(ciphertext);
        debug_log!(
            CRYPTO_LOGGER_DOMAIN,
            "Encryption successful, produced Base64 string"
        );
        Ok(encoded)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use aes::Aes128;
    use aes::cipher::{
        BlockDecryptMut, BlockEncryptMut, KeyIvInit, block_padding::Pkcs7,
        generic_array::GenericArray,
    };
    use base64::{
        Engine,
        engine::general_purpose::{
            STANDARD as BASE64, URL_SAFE_NO_PAD as BASE64_URL_SAFE_NO_PAD,
        },
    };
    use cbc::{Decryptor, Encryptor};
    use serde::{Deserialize, Serialize};

    use super::{AesEncrypt, KeyNormalizer};
    use crate::Error;
    use crate::crypto::AesDecrypt;

    type Aes128CbcEncryptor = Encryptor<Aes128>;
    type Aes128CbcDecryptor = Decryptor<Aes128>;

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct SignPayload {
        expired_at: String,
        uri: String,
    }

    impl SignPayload {
        fn samples() -> [Self; 3] {
            [
                Self {
                    expired_at: "1743400000".into(),
                    uri: "/mnt/media/a.mkv".into(),
                },
                Self {
                    expired_at: "1743400000".into(),
                    uri: "/mnt/media/library/series/season-01/episode-01.mkv"
                        .into(),
                },
                Self {
                    expired_at: "1743400000".into(),
                    uri: "https://example.com:8443/remote/path/with/many/\
                         segments/and/query/like/content/video/original.mkv?\
                         token=abcdef0123456789&part=1"
                        .into(),
                },
            ]
        }

        fn to_current_sign_map(&self) -> BTreeMap<String, String> {
            let mut map = BTreeMap::new();
            map.insert("expired_at".into(), self.expired_at.clone());
            map.insert("uri".into(), self.uri.clone());
            map
        }
    }

    fn encrypt_bytes_to_base64(
        plaintext: &[u8],
        key: &str,
        iv: &str,
        url_safe_no_pad: bool,
    ) -> Result<String, Error> {
        let key = KeyNormalizer::normalize_from_str(key)?;
        let iv = KeyNormalizer::normalize_from_str(iv)?;
        let cipher = Aes128CbcEncryptor::new(
            GenericArray::from_slice(&key),
            GenericArray::from_slice(&iv),
        );

        let mut output = vec![0u8; plaintext.len() + 16];
        let ciphertext = cipher
            .encrypt_padded_b2b_mut::<Pkcs7>(plaintext, &mut output)
            .map_err(|error| Error::EncryptionError(error.to_string()))?;

        let encoded = if url_safe_no_pad {
            BASE64_URL_SAFE_NO_PAD.encode(ciphertext)
        } else {
            BASE64.encode(ciphertext)
        };

        Ok(encoded)
    }

    fn decrypt_base64_to_bytes(
        encrypted: &str,
        key: &str,
        iv: &str,
        url_safe_no_pad: bool,
    ) -> Result<Vec<u8>, Error> {
        let decoded = if url_safe_no_pad {
            BASE64_URL_SAFE_NO_PAD
                .decode(encrypted)
                .map_err(Error::Base64DecodeError)?
        } else {
            BASE64.decode(encrypted).map_err(Error::Base64DecodeError)?
        };

        let key = KeyNormalizer::normalize_from_str(key)?;
        let iv = KeyNormalizer::normalize_from_str(iv)?;
        let cipher = Aes128CbcDecryptor::new(
            GenericArray::from_slice(&key),
            GenericArray::from_slice(&iv),
        );

        let mut buffer = decoded;
        let decrypted = cipher
            .decrypt_padded_mut::<Pkcs7>(&mut buffer)
            .map_err(|error| Error::DecryptionError(error.to_string()))?;

        Ok(decrypted.to_vec())
    }

    #[test]
    fn experimental_msgpack_sign_roundtrip_is_deterministic()
    -> Result<(), Error> {
        let payload = SignPayload {
            expired_at: "1743400000".into(),
            uri: "/mnt/media/library/series/season-01/episode-01.mkv".into(),
        };
        let plaintext = rmp_serde::to_vec_named(&payload)
            .map_err(|error| Error::EncryptionError(error.to_string()))?;

        let encrypted1 = encrypt_bytes_to_base64(
            &plaintext,
            "1234567890123456",
            "1234567890123456",
            true,
        )?;
        let encrypted2 = encrypt_bytes_to_base64(
            &plaintext,
            "1234567890123456",
            "1234567890123456",
            true,
        )?;

        assert_eq!(encrypted1, encrypted2);

        let decrypted = decrypt_base64_to_bytes(
            &encrypted1,
            "1234567890123456",
            "1234567890123456",
            true,
        )?;
        let restored: SignPayload = rmp_serde::from_slice(&decrypted)
            .map_err(|error| Error::DecryptionError(error.to_string()))?;

        assert_eq!(restored, payload);

        Ok(())
    }

    #[test]
    fn experimental_msgpack_sign_is_shorter_than_current_json_sign()
    -> Result<(), Error> {
        for payload in SignPayload::samples() {
            let current_json =
                serde_json::to_vec(&payload.to_current_sign_map())
                    .map_err(Error::JsonError)?;
            let candidate_msgpack = rmp_serde::to_vec_named(&payload)
                .map_err(|error| Error::EncryptionError(error.to_string()))?;

            let current_sign = encrypt_bytes_to_base64(
                &current_json,
                "1234567890123456",
                "1234567890123456",
                false,
            )?;
            let candidate_sign = encrypt_bytes_to_base64(
                &candidate_msgpack,
                "1234567890123456",
                "1234567890123456",
                true,
            )?;

            assert!(
                candidate_sign.len() < current_sign.len(),
                "candidate sign should be shorter for uri={}",
                payload.uri
            );
        }

        Ok(())
    }

    #[test]
    fn aes_encrypt_and_decrypt_roundtrip_with_new_format() -> Result<(), Error>
    {
        let payload = SignPayload {
            expired_at: "1743400000".into(),
            uri: "/mnt/media/library/series/season-01/episode-01.mkv".into(),
        };
        let sign_map: std::collections::HashMap<String, String> =
            payload.to_current_sign_map().into_iter().collect();

        let encrypted = AesEncrypt::encrypt(
            &sign_map,
            "1234567890123456",
            "1234567890123456",
        )?;
        let restored = AesDecrypt::decrypt(
            &encrypted,
            "1234567890123456",
            "1234567890123456",
        )?;

        assert_eq!(restored, sign_map);
        assert!(!encrypted.contains('='));
        assert!(!encrypted.contains('+'));
        assert!(!encrypted.contains('/'));

        Ok(())
    }
}
