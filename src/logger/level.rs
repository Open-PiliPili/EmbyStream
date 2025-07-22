//! Defines the logging levels available in the logging system.
//!
//! The logging levels are ordered from most severe (Off) to least severe (Trace).
//! Each level represents a different severity of log message.

use std::{fmt, str::FromStr};

/// Represents the severity level of a log message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Critical errors that require immediate attention
    Error,

    /// Warning messages for potentially harmful situations
    Warn,

    /// General information about program execution
    Info,

    /// Detailed information useful for debugging
    Debug,

    /// Very detailed information for tracing program flow
    Trace,
}

impl fmt::Display for LogLevel {
    /// Formats the LogLevel for display purposes
    ///
    /// # Arguments
    /// * `f` - The formatter to write to
    ///
    /// # Returns
    /// `fmt::Result` indicating success or failure of the operation
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let level_str = match *self {
            LogLevel::Error => "Error",
            LogLevel::Warn => "Warn",
            LogLevel::Info => "Info",
            LogLevel::Debug => "Debug",
            LogLevel::Trace => "Trace",
        };
        write!(f, "{level_str}")
    }
}

impl FromStr for LogLevel {
    type Err = ();

    /// Creates a LogLevel from a string, defaulting to Info if the string doesn't match
    ///
    /// # Arguments
    /// * `s` - The string to convert to a LogLevel
    ///
    /// # Returns
    /// `Result<LogLevel, ()>` containing the parsed LogLevel or an error if parsing fails
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(LogLevel::Error),
            "warn" => Ok(LogLevel::Warn),
            "info" => Ok(LogLevel::Info),
            "debug" => Ok(LogLevel::Debug),
            "trace" => Ok(LogLevel::Trace),
            _ => Ok(LogLevel::Info),
        }
    }
}
