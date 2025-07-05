use std::io::Error as IoError;

use hyper::http::{Error as HttpError, uri::InvalidUri};
use reqwest::Error as ReqwestError;
use thiserror::Error;

/// Custom error type for configuration operations.
#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] IoError),

    #[error("Invalid URI: {0}")]
    InvalidUri(#[from] InvalidUri),

    #[error("Failed to build HTTP response: {0}")]
    HttpError(#[from] HttpError),

    #[error("Remote request failed: {0}")]
    ReqwestError(#[from] ReqwestError),
}