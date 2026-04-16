use chrono::Duration;

#[derive(Clone, Debug)]
pub struct TokenRequest {
    pub reason: &'static str,
    pub min_valid_for: Duration,
    pub force_refresh: bool,
}

impl TokenRequest {
    pub fn new(reason: &'static str, min_valid_for: Duration) -> Self {
        Self {
            reason,
            min_valid_for,
            force_refresh: false,
        }
    }

    pub fn force_refresh(reason: &'static str) -> Self {
        Self {
            reason,
            min_valid_for: Duration::zero(),
            force_refresh: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TokenSnapshot {
    pub token: crate::oauthutil::OAuthToken,
    pub source: &'static str,
}
