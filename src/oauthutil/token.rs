use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct OAuthToken {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub expiry: Option<DateTime<Utc>>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

impl OAuthToken {
    pub fn has_access_token(&self) -> bool {
        !self.access_token.trim().is_empty()
    }

    pub fn has_refresh_token(&self) -> bool {
        !self.refresh_token.trim().is_empty()
    }

    pub fn authorization_header_value(&self) -> Option<String> {
        let access_token = self.access_token.trim();
        if access_token.is_empty() {
            return None;
        }

        let token_type = self.token_type.trim();
        if token_type.is_empty() {
            return Some(access_token.to_string());
        }

        Some(format!("{token_type} {access_token}"))
    }

    pub fn remaining_lifetime(&self, now: DateTime<Utc>) -> Option<Duration> {
        self.expiry.map(|expiry| expiry - now)
    }

    pub fn is_valid_for(
        &self,
        min_valid_for: Duration,
        now: DateTime<Utc>,
    ) -> bool {
        self.has_access_token()
            && self
                .remaining_lifetime(now)
                .map(|remaining| remaining > min_valid_for)
                .unwrap_or(false)
    }

    pub fn from_refresh_parts(
        access_token: String,
        refresh_token: String,
        token_type: String,
        expiry: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: if token_type.trim().is_empty() {
                default_token_type()
            } else {
                token_type
            },
            expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};

    use super::OAuthToken;

    #[test]
    fn token_validity_requires_access_token_and_expiry() {
        let now = Utc
            .with_ymd_and_hms(2026, 4, 16, 12, 0, 0)
            .single()
            .expect("valid timestamp");
        let token = OAuthToken::from_refresh_parts(
            "access-token".to_string(),
            "refresh-token".to_string(),
            "Bearer".to_string(),
            Some(now + Duration::minutes(30)),
        );

        assert!(token.is_valid_for(Duration::minutes(5), now));
        assert!(!token.is_valid_for(Duration::minutes(40), now));
    }
}
