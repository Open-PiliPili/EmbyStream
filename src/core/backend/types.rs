#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub crypto_key: String,
    pub crypto_iv: String,
    pub user_agent: Option<String>,
}