use std::{io::Error as IoError, sync::Arc};

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("IO error: {0}")]
    IoError(#[from] Arc<IoError>),
}
