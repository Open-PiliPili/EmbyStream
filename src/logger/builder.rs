use std::fmt::Debug;
use time::UtcOffset;

use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Registry,
};

use super::{LogLevel, LogRotation};

/// A logging system that can be configured via the builder pattern
#[derive(Debug)]
pub struct Logger;

impl Logger {
    /// Creates a new LoggerBuilder for configuring the logging system
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }
}

/// A builder for configuring and initializing a logging system
///
/// Provides a fluent interface for setting up both file and console logging
/// with customizable formatting, rotation, and filtering.
#[derive(Debug, Clone)]
pub struct LoggerBuilder {
    /// Maximum log level to capture (inclusive)
    max_level: LogLevel,

    /// Directory where log files will be stored
    directory: String,

    /// Prefix for log file names
    file_name_prefix: String,

    /// Rotation strategy for log files
    rolling: LogRotation,
}

impl Default for LoggerBuilder {
    /// Creates a default LoggerBuilder configuration:
    /// - LogLevel::Info
    /// - "logs" directory
    /// - No file prefix
    /// - Daily rotation
    fn default() -> Self {
        Self {
            max_level: LogLevel::Info,
            directory: "logs".to_owned(),
            file_name_prefix: "".to_owned(),
            rolling: LogRotation::Daily,
        }
    }
}

impl LoggerBuilder {
    /// Sets the maximum log level to capture
    ///
    /// # Arguments
    /// * `level` - The maximum level to log (inclusive)
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.max_level = level;
        self
    }

    /// Sets the directory for log file storage
    ///
    /// # Arguments
    /// * `directory` - Path to the log directory
    ///
    /// # Notes
    /// - Directory will be created if it doesn't exist
    /// - Relative paths are resolved from current working directory
    pub fn with_directory(mut self, directory: &str) -> Self {
        self.directory = directory.to_owned();
        self
    }

    /// Sets the prefix for log file names
    ///
    /// # Arguments
    /// * `file_prefix` - Prefix to prepend to log filenames
    ///
    /// # Example
    /// "myapp" prefix creates files like "myapp-2023-01-01.log"
    pub fn with_file_prefix(mut self, file_prefix: &str) -> Self {
        self.file_name_prefix = file_prefix.to_owned();
        self
    }

    /// Sets the log rotation strategy
    ///
    /// # Arguments
    /// * `rolling` - The rotation strategy (Daily, Hourly, Never)
    ///
    /// # Notes
    /// - Affects both file naming and rotation behavior
    pub fn with_rolling(mut self, rolling: LogRotation) -> Self {
        self.rolling = rolling;
        self
    }

    /// Initializes the global logger with the configured settings
    ///
    /// # Panics
    /// - If time format parsing fails
    /// - If logger initialization fails
    ///
    /// # Notes
    /// - Should only be called once per application
    /// - Configures both file and console logging
    /// - File logging includes:
    ///   - Compact format
    ///   - Precise timestamps
    ///   - No ANSI colors
    /// - Console logging includes:
    ///   - Compact format
    ///   - ANSI colors
    ///   - Same timestamps as files
    pub fn build(self) {
        let timer_fmt = time::format_description::parse(
            "[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second].[subsecond digits:6]",
        )
            .expect("Failed to parse time format");
        let time_offset = UtcOffset::current_local_offset()
            .unwrap_or_else(|_| UtcOffset::UTC);
        let timer = fmt::time::OffsetTime::new(time_offset, timer_fmt);

        // Try to get filter from env, fallback to configured level
        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(self.max_level.to_string()));

        // Configure file appender with rotation
        let file_appender = self.rolling
            .create_file_appender(self.directory, self.file_name_prefix);

        // File logging layer
        let file_layer = fmt::Layer::new()
            .compact()
            .with_ansi(false)
            .with_timer(timer.clone())
            .with_level(true)
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_names(false)
            .with_thread_ids(false)
            .with_writer(file_appender);

        // Console logging layer
        let console_layer = fmt::Layer::new()
            .compact()
            .with_ansi(true)
            .with_timer(timer)
            .with_level(true)
            .with_target(false)
            .with_file(true)
            .with_line_number(true)
            .with_thread_names(false)
            .with_thread_ids(false);

        // Initialize global logger
        Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    }
}