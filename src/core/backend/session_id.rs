use uuid::Uuid;

fn generate_uuid() -> String {
    Uuid::new_v4().hyphenated().to_string().to_uppercase()
}

pub fn generate_stream_session_id() -> String {
    generate_uuid()
}

pub fn generate_playback_session_id() -> String {
    generate_uuid()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const UUID_LENGTH: usize = 36;

    #[test]
    fn session_id_format_is_valid() {
        let id = generate_stream_session_id();
        assert_eq!(id.len(), UUID_LENGTH);
        assert_eq!(id.matches('-').count(), 4);
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
    fn playback_session_id_format_is_valid() {
        let id = generate_playback_session_id();
        assert_eq!(id.len(), UUID_LENGTH);
        assert_eq!(id.matches('-').count(), 4);
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
