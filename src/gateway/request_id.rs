use uuid::Uuid;

pub fn generate_request_id() -> String {
    Uuid::new_v4().hyphenated().to_string().to_uppercase()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::generate_request_id;

    const UUID_LENGTH: usize = 36;

    #[test]
    fn request_id_format_is_valid() {
        let request_id = generate_request_id();
        assert_eq!(request_id.len(), UUID_LENGTH);
        assert_eq!(request_id.matches('-').count(), 4);
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
    fn request_id_is_uuid_sized() {
        let request_id = generate_request_id();
        assert_eq!(request_id.len(), UUID_LENGTH);
    }
}
