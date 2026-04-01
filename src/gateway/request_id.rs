use std::sync::atomic::{AtomicU64, Ordering};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

const TIMESTAMP_MODULO: u128 = 1_000_000_000;
const REQUEST_ID_PREFIX: &str = "request:id";
const TIMESTAMP_HEX_WIDTH: usize = 12;
const COUNTER_HEX_WIDTH: usize = 8;

pub fn generate_request_id() -> String {
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() % TIMESTAMP_MODULO)
        .unwrap_or(0);

    format!(
        "{REQUEST_ID_PREFIX}:{timestamp_ms:0TIMESTAMP_HEX_WIDTH$x}:{counter:0COUNTER_HEX_WIDTH$x}"
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::generate_request_id;

    const MAX_REQUEST_ID_LENGTH: usize = 40;

    #[test]
    fn request_id_format_is_valid() {
        let request_id = generate_request_id();
        assert!(request_id.starts_with("request:id:"));
        assert_eq!(request_id.matches(':').count(), 3);
    }

    #[test]
    fn request_ids_are_unique() {
        let mut ids = HashSet::new();
        for _ in 0..1000 {
            let request_id = generate_request_id();
            assert!(ids.insert(request_id), "duplicate request id generated");
        }
    }

    #[test]
    fn request_id_is_short() {
        let request_id = generate_request_id();
        assert!(
            request_id.len() < MAX_REQUEST_ID_LENGTH,
            "request id too long: {request_id}"
        );
    }
}
