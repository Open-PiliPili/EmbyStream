use thiserror::Error;

use crate::Error as CommonError;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Empty signature")]
    EmptySignature,
    #[error("Invalid media source")]
    InvalidMediaSource,
    #[error("Invalid encrypted signature")]
    InvalidEncryptedSignature,
    #[error("Common error: {0}")]
    CommonError(#[from] CommonError),
    #[error("Invalid uri")]
    InvalidUri,
    #[error("Expired stream")]
    ExpiredStream,
    #[error("Encrypted signature failed")]
    EncryptSignatureFailed,
    #[error("System time error: {0}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
    #[error("Invalid strm file")]
    InvalidStrmFile,
    #[error("Empty strm file")]
    EmptyStrmFile,
    #[error("Strm file too large")]
    StrmFileTooLarge,
    #[error("Strm file IO error: {0}")]
    StrmFileIoError(String),
}