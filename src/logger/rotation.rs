//! Defines the log rotation strategies for file-based logging.
//!
//! This module provides different rotation strategies for log files,
//! allowing for automatic file management based on time intervals.

use tracing_appender::rolling::{self, RollingFileAppender};

/// Defines how often log files should be rotated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogRotation {
    /// Rotate log files every minute
    Minutely,

    /// Rotate log files every hour
    Hourly,

    /// Rotate log files every day
    Daily,

    /// Never rotate log files
    Never,
}

impl LogRotation {
    /// Creates a new file appender with the specified rotation strategy.
    ///
    /// # Arguments
    ///
    /// * `directory` - The directory where log files will be stored
    /// * `file_prefix` - The prefix to use for log file names
    ///
    /// # Returns
    ///
    /// A `RollingFileAppender` configured with the specified rotation strategy
    pub fn create_file_appender(
        self,
        directory: String,
        file_prefix: String,
    ) -> RollingFileAppender {
        match self {
            LogRotation::Minutely => rolling::minutely(directory, file_prefix),
            LogRotation::Hourly => rolling::hourly(directory, file_prefix),
            LogRotation::Daily => rolling::daily(directory, file_prefix),
            LogRotation::Never => rolling::never(directory, file_prefix),
        }
    }
}
