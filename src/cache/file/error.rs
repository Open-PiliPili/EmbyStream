use std::io::Error as IoError;

use thiserror::Error;

/// Custom error type for configuration operations.
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(IoError),
}
