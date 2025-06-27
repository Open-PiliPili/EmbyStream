#[allow(dead_code)]

/// Default domain used for logging macros when no custom domain is provided.
/// This helps categorize log messages consistently across the application.
pub const DEFAULT_LOGGER_DOMAIN: &str = "GENERAL";

pub const CONFIG_LOGGER_DOMAIN: &str = "CONFIG";

pub const NETWORK_LOGGER_DOMAIN: &str = "NETWORK";

pub const CRYPTO_LOGGER_DOMAIN: &str = "CRYPTO";

pub const CRYPTO_CACHE_LOGGER_DOMAIN: &str = "CRYPTO-CACHE";

pub const FILE_CACHE_LOGGER_DOMAIN: &str = "FILE-CACHE";