use std::fmt::Debug;
use time::UtcOffset;

use tracing_subscriber::{
    EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt,
};

use super::{LogLevel, LogRotation};

#[derive(Debug)]
pub struct Logger;

impl Logger {
    pub fn builder() -> LoggerBuilder {
        LoggerBuilder::default()
    }
}

#[derive(Debug, Clone)]
pub struct LoggerBuilder {
    max_level: LogLevel,
    directory: String,
    file_name_prefix: String,
    rolling: LogRotation,
}

impl Default for LoggerBuilder {
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
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.max_level = level;
        self
    }

    pub fn with_directory(mut self, directory: &str) -> Self {
        self.directory = directory.to_owned();
        self
    }

    pub fn with_file_prefix(mut self, file_prefix: &str) -> Self {
        self.file_name_prefix = file_prefix.to_owned();
        self
    }

    pub fn with_rolling(mut self, rolling: LogRotation) -> Self {
        self.rolling = rolling;
        self
    }

    pub fn build(self) {
        let timer_fmt = time::format_description::parse(
            "[year]-[month padding:zero]-[day padding:zero] [hour]:[minute]:[second].[subsecond digits:6]",
        )
            .expect("Failed to parse time format");
        let time_offset =
            UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
        let timer = fmt::time::OffsetTime::new(time_offset, timer_fmt);

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(self.max_level.to_string()));

        let file_appender = self
            .rolling
            .create_file_appender(self.directory, self.file_name_prefix);

        let is_debug = self.max_level == LogLevel::Debug;

        let file_layer = fmt::Layer::new()
            .compact()
            .with_ansi(false)
            .with_timer(timer.clone())
            .with_target(false)
            .with_file(is_debug)
            .with_line_number(is_debug)
            .with_thread_names(false)
            .with_thread_ids(false)
            .with_writer(file_appender);

        let console_layer = fmt::Layer::new()
            .compact()
            .with_ansi(true)
            .with_timer(timer)
            .with_target(false)
            .with_file(is_debug)
            .with_line_number(is_debug)
            .with_thread_names(false)
            .with_thread_ids(false);

        Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    }
}
