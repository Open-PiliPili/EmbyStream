use std::fmt::Debug;
use time::UtcOffset;
use time::macros::format_description;

use chrono::Utc;
use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt,
    registry::LookupSpan, util::SubscriberInitExt,
};

use crate::{log_stream::LogStreamHub, web::contracts::LogEntry};

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
    live_logs: Option<(LogStreamHub, String)>,
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        Self {
            max_level: LogLevel::Info,
            directory: "logs".to_owned(),
            file_name_prefix: "".to_owned(),
            rolling: LogRotation::Daily,
            live_logs: None,
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

    pub fn with_live_logs(
        mut self,
        hub: LogStreamHub,
        source: impl Into<String>,
    ) -> Self {
        self.live_logs = Some((hub, source.into()));
        self
    }

    pub fn build(self) {
        let timer_fmt = format_description!(
            "[year]-[month padding:zero]-[day padding:zero] \
             [hour]:[minute]:[second].[subsecond digits:6]"
        );
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

        let live_log_layer = self
            .live_logs
            .map(|(hub, source)| LiveLogLayer { hub, source });

        let subscriber = Registry::default()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer);

        if let Some(live_log_layer) = live_log_layer {
            subscriber.with(live_log_layer).init();
        } else {
            subscriber.init();
        }
    }
}

#[derive(Debug, Clone)]
struct LiveLogLayer {
    hub: LogStreamHub,
    source: String,
}

impl<S> Layer<S> for LiveLogLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(
        &self,
        event: &Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = LogMessageVisitor::default();
        event.record(&mut visitor);

        let message = visitor.message();
        if message.is_empty() {
            return;
        }

        self.hub.publish(LogEntry {
            timestamp: Utc::now(),
            level: event.metadata().level().as_str().to_string(),
            source: self.source.clone(),
            message,
        });
    }
}

#[derive(Debug, Default)]
struct LogMessageVisitor {
    message: Option<String>,
    extras: Vec<String>,
}

impl LogMessageVisitor {
    fn record_formatted(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            self.message = Some(value);
            return;
        }

        self.extras.push(format!("{}={value}", field.name()));
    }

    fn message(self) -> String {
        match (self.message, self.extras.is_empty()) {
            (Some(message), true) => message,
            (Some(message), false) => {
                format!("{message} {}", self.extras.join(" "))
            }
            (None, _) => self.extras.join(" "),
        }
    }
}

impl Visit for LogMessageVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_formatted(field, value.to_string());
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        self.record_formatted(field, format!("{value:?}"));
    }
}
