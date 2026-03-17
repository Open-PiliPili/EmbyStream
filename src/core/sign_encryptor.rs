use crate::{
    AppState, STREAM_LOGGER_DOMAIN,
    core::{error::Error as AppStreamError, sign::Sign},
    crypto::{Crypto, CryptoInput, CryptoOperation, CryptoOutput},
    debug_log,
};

pub struct SignEncryptor;

impl SignEncryptor {
    pub async fn encrypt(
        sign: &Sign,
        state: &AppState,
    ) -> Result<String, AppStreamError> {
        let sign_map = sign.to_map();
        debug_log!(STREAM_LOGGER_DOMAIN, "Encrypting sign map: {:?}", sign_map);

        let config = state.get_config().await;
        let crypto_result = Crypto::execute(
            CryptoOperation::Encrypt,
            CryptoInput::Dictionary(sign_map),
            &config.general.encipher_key,
            &config.general.encipher_iv,
        )
        .map_err(AppStreamError::CommonError)?;

        match crypto_result {
            CryptoOutput::Encrypted(value) => Ok(value),
            _ => Err(AppStreamError::EncryptSignatureFailed),
        }
    }
}
