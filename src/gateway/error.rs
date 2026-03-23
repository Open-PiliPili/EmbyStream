use std::io::Error as IoError;

use hyper::Error as HyperError;
use hyper::http::{Error as HttpError, uri::InvalidUri};
use hyper_util::client::legacy::Error as HyperUtilClientError;
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

    #[error("Hyper error: {0}")]
    Hyper(#[from] HyperError),

    #[error("Hyper client error: {0}")]
    HyperUtilClient(#[from] HyperUtilClientError),
}
