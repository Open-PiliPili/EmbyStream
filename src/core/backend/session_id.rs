use std::sync::atomic::{AtomicU64, Ordering};

/// Fast session ID generator using atomic counter.
/// Replaces Uuid::new_v4() to avoid random number generation
/// and heap allocation overhead (~1-5μs per call).
///
/// # Overflow Safety
/// AtomicU64 wraps around on overflow (18,446,744,073,709,551,615 -> 0).
/// At 1 million requests/sec, this takes ~584,942 years to overflow.
/// Combined with timestamp prefix, IDs remain unique even after wrap.
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);
static PLAYBACK_SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

const TIMESTAMP_MODULO: u128 = 1_000_000_000;

fn generate_prefixed_session_id(prefix: &str, counter: &AtomicU64) -> String {
    let counter = counter.fetch_add(1, Ordering::Relaxed);
    let start_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() % TIMESTAMP_MODULO)
        .unwrap_or(0);
    format!("{prefix}{start_ms}-{counter}")
}

/// Generates a unique session ID for stream correlation.
/// Format: "s{process_start_ms}-{counter}" for easy grep and uniqueness.
pub fn generate_stream_session_id() -> String {
    generate_prefixed_session_id("s", &SESSION_COUNTER)
}

pub fn generate_playback_session_id() -> String {
    generate_prefixed_session_id("play-", &PLAYBACK_SESSION_COUNTER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const MAX_SESSION_ID_LENGTH: usize = 30;

    #[test]
    fn session_id_format_is_valid() {
        let id = generate_stream_session_id();
        assert!(id.starts_with('s'));
        assert!(id.contains('-'));
    }

    #[test]
    fn session_ids_are_unique() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = generate_stream_session_id();
            assert!(ids.insert(id), "Duplicate session ID generated");
        }
    }

    #[test]
    fn session_id_is_short() {
        let id = generate_stream_session_id();
        assert!(
            id.len() < MAX_SESSION_ID_LENGTH,
            "Session ID too long: {}",
            id
        );
    }

    #[test]
    fn playback_session_id_format_is_valid() {
        let id = generate_playback_session_id();
        assert!(id.starts_with("play-"));
        assert_eq!(id.matches('-').count(), 2);
    }

    #[test]
    fn playback_session_ids_are_unique() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let id = generate_playback_session_id();
            assert!(ids.insert(id), "Duplicate playback session ID");
        }
    }
}
