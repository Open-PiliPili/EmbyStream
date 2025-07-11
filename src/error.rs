use thiserror::Error;

/// Custom error type for configuration operations.
#[derive(Error, Debug)]
pub enum Error {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Error parsing TOML content.
    #[error("TOML parse error: {0}")]
    TomlParseError(#[from] toml::de::Error),

    /// Invalid encipher key length.
    #[error("Encipher key must be 16 bytes, got {0} bytes")]
    InvalidEncipherKey(usize),

    /// Missing [General] section in config file.
    #[error("No [General] section found in config file")]
    MissingGeneralSection,

    /// Backend configuration does not match backend type.
    #[error("Invalid backend configuration for backend type: {0}")]
    InvalidBackendConfig(String),

    /// JSON serialization or deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Base64 decoding error.
    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),

    /// Encryption error.
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    /// Decryption error.
    #[error("Decryption error: {0}")]
    DecryptionError(String),

    #[error("Load config error: {0}")]
    LoadConfigError(String),
}
