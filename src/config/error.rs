use std::io::Error as IoError;

use thiserror::Error;
use toml::de::Error as TomlError;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Home directory not found")]
    NoHomeDir,
    #[error("File I/O error: {0}")]
    Io(#[from] IoError),
    #[error("TOML parse error: {0}")]
    Toml(#[from] TomlError),
    #[error("Failed to create config directory '{path}': {source}")]
    CreateDir { path: String, source: IoError },
    #[error("Failed to copy template file: {0}")]
    CopyTemplate(IoError),
    #[error("Invalid backend type: '{0}'")]
    InvalidBackendType(String),
    #[error("Missing configuration: '{0}'")]
    MissingConfig(String),
}
