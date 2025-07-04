use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
}
