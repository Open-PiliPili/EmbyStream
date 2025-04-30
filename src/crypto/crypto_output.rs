use std::fmt;
use std::collections::HashMap;

/// Output type for cryptographic operations.
#[derive(Debug)]
pub enum CryptoOutput {
    Encrypted(String),
    Dictionary(HashMap<String, String>),
}

impl fmt::Display for CryptoOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoOutput::Encrypted(s) => {
                // Truncate long strings for readability
                if s.len() > 50 {
                    write!(f, "Encrypted({}...)", &s[..50])
                } else {
                    write!(f, "Encrypted({})", s)
                }
            }
            CryptoOutput::Dictionary(dict) => {
                write!(f, "Dictionary {{ ")?;
                let mut first = true;
                for (key, value) in dict {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                    first = false;
                }
                write!(f, " }}")
            }
        }
    }
}
