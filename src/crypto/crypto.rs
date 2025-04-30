use super::{
    aes_encrypt::AesEncrypt, aes_decrypt::AesDecrypt,
    crypto_operation::CryptoOperation, crypto_input::CryptoInput,
    crypto_output::CryptoOutput
};

use crate::{CRYPTO_LOGGER_DOMAIN, Error, error_log, info_log};

/// Unified cryptographic operation handler.
pub struct Crypto;

impl Crypto {
    /// Performs a cryptographic operation (encrypt or decrypt) based on the operation type.
    ///
    /// # Arguments
    ///
    /// * `operation` - The operation to perform (Encrypt or Decrypt).
    /// * `input` - For Encrypt: HashMap<String, String>; for Decrypt: Base64-encoded string.
    /// * `key` - The 16-byte encryption/decryption key.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` - For Encrypt: Base64-encoded encrypted string.
    /// * `Ok(HashMap<String, String>)` - For Decrypt: Decrypted dictionary.
    /// * `Err(Error)` - If the operation fails.
    pub fn execute(
        operation: CryptoOperation,
        input: CryptoInput,
        key: &str
    ) -> Result<CryptoOutput, Error> {
        info_log!(CRYPTO_LOGGER_DOMAIN, "Executing cryptographic operation: {:?}", operation);

        match operation {
            CryptoOperation::Encrypt => {
                let dict = match input {
                    CryptoInput::Dictionary(dict) => dict,
                    _ => {
                        error_log!(CRYPTO_LOGGER_DOMAIN, "Invalid input for encryption: expected dictionary");
                        return Err(Error::EncryptionError("Invalid input: expected dictionary".to_string()));
                    }
                };
                let encrypted = AesEncrypt::encrypt(&dict, key)?;
                Ok(CryptoOutput::Encrypted(encrypted))
            }
            CryptoOperation::Decrypt => {
                let encrypted = match input {
                    CryptoInput::Encrypted(encrypted) => encrypted,
                    _ => {
                        error_log!(
                            CRYPTO_LOGGER_DOMAIN,
                            "Invalid input for decryption: expected encrypted string"
                        );
                        return Err(
                            Error::DecryptionError(
                                "Invalid input: expected encrypted string".to_string()
                            )
                        );
                    }
                };
                let dict = AesDecrypt::decrypt(&encrypted, key)?;
                Ok(CryptoOutput::Dictionary(dict))
            }
        }
    }
}