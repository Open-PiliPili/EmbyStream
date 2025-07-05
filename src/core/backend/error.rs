use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid signature")]
    InvalidSignature,
}