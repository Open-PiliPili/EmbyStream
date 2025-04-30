//! Defines the logging levels available in the logging system.
//!
//! The logging levels are ordered from most severe (Off) to least severe (Trace).
//! Each level represents a different severity of log message.

use std::fmt;

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
        write!(f, "{}", level_str)
    }
}
