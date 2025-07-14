use crate::config::backend::{
    Backend,
    types::BackendConfig as StreamBackendConfig
};

#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub crypto_key: String,
    pub crypto_iv: String,
    pub backend: Backend,
    pub backend_config: StreamBackendConfig,
}
