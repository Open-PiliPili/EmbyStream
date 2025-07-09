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
}
