use std::fmt;

/// Cryptographic operation type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CryptoOperation {
    Encrypt,
    Decrypt,
}

impl fmt::Display for CryptoOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoOperation::Encrypt => write!(f, "Encrypt"),
            CryptoOperation::Decrypt => write!(f, "Decrypt"),
        }
    }
}
