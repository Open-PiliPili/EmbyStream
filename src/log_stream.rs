use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, OnceLock},
};

use crate::web::contracts::LogEntry;
use tokio::sync::broadcast;

const LOG_CHANNEL_CAPACITY: usize = 2048;
const LOG_RECENT_CAPACITY: usize = 1000;

#[derive(Debug, Clone, Default)]
pub struct LogStreamFilter {
    pub source: Option<String>,
    pub level: Option<String>,
}

impl LogStreamFilter {
    pub fn matches(&self, entry: &LogEntry) -> bool {
        let matches_source = self
            .source
            .as_deref()
            .map(|source| entry.source == source)
            .unwrap_or(true);
        let matches_level = self
            .level
            .as_deref()
            .map(|level| entry.level.eq_ignore_ascii_case(level))
            .unwrap_or(true);

        matches_source && matches_level
    }
}

#[derive(Debug, Clone)]
pub struct LogStreamHub {
    sender: broadcast::Sender<LogEntry>,
    recent: Arc<Mutex<VecDeque<LogEntry>>>,
    recent_capacity: usize,
}

impl LogStreamHub {
    pub fn new(channel_capacity: usize, recent_capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(channel_capacity);
        Self {
            sender,
            recent: Arc::new(Mutex::new(VecDeque::with_capacity(
                recent_capacity,
            ))),
            recent_capacity,
        }
    }

    pub fn publish(&self, entry: LogEntry) {
        {
            let mut recent = self
                .recent
                .lock()
                .expect("recent log buffer should not be poisoned");
            recent.push_back(entry.clone());
            while recent.len() > self.recent_capacity {
                recent.pop_front();
            }
        }

        let _ = self.sender.send(entry);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.sender.subscribe()
    }

    pub fn snapshot(
        &self,
        filter: &LogStreamFilter,
        limit: usize,
    ) -> Vec<LogEntry> {
        let recent = self
            .recent
            .lock()
            .expect("recent log buffer should not be poisoned");

        recent
            .iter()
            .rev()
            .filter(|entry| filter.matches(entry))
            .take(limit)
            .cloned()
            .collect()
    }
}

static GLOBAL_LOG_STREAM: OnceLock<LogStreamHub> = OnceLock::new();

pub fn global_log_stream() -> LogStreamHub {
    GLOBAL_LOG_STREAM
        .get_or_init(|| {
            LogStreamHub::new(LOG_CHANNEL_CAPACITY, LOG_RECENT_CAPACITY)
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::{LogStreamFilter, LogStreamHub};
    use crate::web::contracts::LogEntry;

    fn test_entry(
        source: &str,
        level: &str,
        message: &str,
        seconds_offset: i64,
    ) -> LogEntry {
        LogEntry {
            timestamp: Utc::now() + chrono::Duration::seconds(seconds_offset),
            level: level.to_string(),
            source: source.to_string(),
            message: message.to_string(),
        }
    }

    #[test]
    fn snapshot_respects_capacity_and_returns_newest_first() {
        let hub = LogStreamHub::new(16, 2);
        hub.publish(test_entry("stream", "INFO", "first", 0));
        hub.publish(test_entry("stream", "INFO", "second", 1));
        hub.publish(test_entry("stream", "INFO", "third", 2));

        let items = hub.snapshot(&LogStreamFilter::default(), 10);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].message, "third");
        assert_eq!(items[1].message, "second");
    }

    #[test]
    fn snapshot_filters_by_source_and_level() {
        let hub = LogStreamHub::new(16, 10);
        hub.publish(test_entry("stream", "INFO", "keep-source", 0));
        hub.publish(test_entry("runtime", "INFO", "drop-source", 1));
        hub.publish(test_entry("stream", "ERROR", "keep-level", 2));

        let source_only = hub.snapshot(
            &LogStreamFilter {
                source: Some("stream".to_string()),
                level: None,
            },
            10,
        );
        assert_eq!(source_only.len(), 2);

        let source_and_level = hub.snapshot(
            &LogStreamFilter {
                source: Some("stream".to_string()),
                level: Some("error".to_string()),
            },
            10,
        );
        assert_eq!(source_and_level.len(), 1);
        assert_eq!(source_and_level[0].message, "keep-level");
    }

    #[tokio::test]
    async fn publish_broadcasts_to_subscribers() {
        let hub = LogStreamHub::new(16, 10);
        let mut receiver = hub.subscribe();
        let entry = test_entry("audit", "INFO", "broadcast", 0);
        hub.publish(entry.clone());

        let received = receiver.recv().await.expect("broadcast entry");
        assert_eq!(received.source, entry.source);
        assert_eq!(received.level, entry.level);
        assert_eq!(received.message, entry.message);
    }
}
