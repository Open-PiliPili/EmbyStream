use std::io::Error as IoError;

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
    #[error("Empty emby token")]
    EmptyEmbyToken,
    #[error("Empty emby device id")]
    EmptyEmbyDeviceId,
    #[error("Emby path request error")]
    EmbyPathRequestError,
    #[error("Emby path parser error")]
    EmbyPathParserError,
    #[error("Invalid openlist uri: {0}")]
    InvalidOpenListUri(String),
    #[error("Unexpected openlist error: {0}")]
    UnexpectedOpenListError(String),
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("IO error: {0}")]
    IoError(#[from] IoError),
    #[error("Backend node not found in request")]
    BackendNodeNotFound,
    #[error("WebDav upstream URL: {0}")]
    WebDavUrl(String),
    #[error(
        "Disk backend must use a local base_url; use type StreamRelay to forward \
         signed streams to a remote host"
    )]
    DiskRemoteNotSupported,
    #[error(
        "StreamRelay base_url must be a non-loopback remote host \
         (not empty, 127.0.0.1, localhost, or 0.0.0.0)"
    )]
    StreamRelayForbiddenLocalTarget,
}
