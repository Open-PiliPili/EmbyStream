use std::{
    sync::Arc,
    io::Error as IoError
};

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] Arc<IoError>),
}