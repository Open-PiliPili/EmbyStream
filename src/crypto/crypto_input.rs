use std::collections::HashMap;
use std::fmt;

/// Input type for cryptographic operations.
#[derive(Debug)]
pub enum CryptoInput {
    Dictionary(HashMap<String, String>),
    Encrypted(String),
}

impl fmt::Display for CryptoInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoInput::Dictionary(dict) => {
                write!(f, "Dictionary {{ ")?;
                let mut first = true;
                for (key, value) in dict {
                    if !first {
                        write!(f, ", ")?;
                    }
                    write!(f, "{key}: {value}")?;
                    first = false;
                }
                write!(f, " }}")
            }
            CryptoInput::Encrypted(s) => {
                // Truncate long strings for readability
                if s.len() > 50 {
                    write!(f, "Encrypted({}...)", &s[..50])
                } else {
                    write!(f, "Encrypted({s})")
                }
            }
        }
    }
}
