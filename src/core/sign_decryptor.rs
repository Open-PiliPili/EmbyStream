use crate::{
    AppState, STREAM_LOGGER_DOMAIN,
    core::{error::Error as AppStreamError, sign::Sign},
    crypto::{Crypto, CryptoInput, CryptoOperation, CryptoOutput},
    debug_log, info_log,
    sign::SignParams,
    util::StringUtil,
};

pub struct SignDecryptor;

impl SignDecryptor {
    pub async fn decrypt(
        sign_str: &str,
        params: &SignParams,
        state: &AppState,
    ) -> Result<Sign, AppStreamError> {
        if sign_str.is_empty() {
            return Err(AppStreamError::EmptySignature);
        }

        let decrypt_cache = state.get_decrypt_cache().await;
        let cache_key = Self::build_cache_key(params)?;

        if let Some(sign) = decrypt_cache.get(&cache_key) {
            debug_log!(
                STREAM_LOGGER_DOMAIN,
                "sign_decrypt_cache_hit key={} sign={:?}",
                cache_key,
                sign
            );
            return Ok(sign);
        }

        let config = state.get_config().await;
        let crypto_result = Crypto::execute(
            CryptoOperation::Decrypt,
            CryptoInput::Encrypted(sign_str.to_string()),
            &config.general.encipher_key,
            &config.general.encipher_iv,
        )
        .map_err(AppStreamError::CommonError)?;

        match crypto_result {
            CryptoOutput::Encrypted(_) => {
                Err(AppStreamError::InvalidEncryptedSignature)
            }
            CryptoOutput::Dictionary(sign_map) => {
                debug_log!(
                    STREAM_LOGGER_DOMAIN,
                    "sign_decrypt_ready key={} sign={:?}",
                    cache_key,
                    sign_map
                );
                decrypt_cache.insert(cache_key.clone(), sign_map.clone());
                info_log!(
                    STREAM_LOGGER_DOMAIN,
                    "sign_decrypt_cache_store key={}",
                    cache_key
                );
                Ok(Sign::from_map(&sign_map))
            }
        }
    }

    fn build_cache_key(params: &SignParams) -> Result<String, AppStreamError> {
        if params.sign.is_empty() {
            return Err(AppStreamError::InvalidEncryptedSignature);
        }
        let input = params.sign.to_lowercase();
        let sign_hash = StringUtil::hash_hex(&input);
        Ok(format!("stream:sign_decrypt:sign_hash:{sign_hash}"))
    }
}

#[cfg(test)]
mod tests {
    use super::SignDecryptor;
    use crate::core::{error::Error as AppStreamError, sign::SignParams};

    #[test]
    fn decrypt_cache_key_is_structured() {
        let params = SignParams {
            sign: "EncryptedSignValue".into(),
            device_id: String::new(),
        };

        let key = SignDecryptor::build_cache_key(&params);

        assert!(key.is_ok());
        assert!(
            key.unwrap_or_default()
                .starts_with("stream:sign_decrypt:sign_hash:")
        );
    }

    #[test]
    fn decrypt_cache_key_rejects_empty_sign() {
        let params = SignParams {
            sign: String::new(),
            device_id: String::new(),
        };

        let key = SignDecryptor::build_cache_key(&params);

        assert!(matches!(
            key,
            Err(AppStreamError::InvalidEncryptedSignature)
        ));
    }
}
