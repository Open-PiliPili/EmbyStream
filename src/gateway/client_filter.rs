use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode, body::Incoming, header};
use tokio::sync::OnceCell;

use crate::{
    AppState, CLIENT_FILTER_LOGGER_DOMAIN, debug_log, error_log, info_log,
};
use crate::{
    config::general::UserAgent,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

const HEADER_CLIENT_KEY: &str = "Client";

#[derive(Clone)]
pub struct ClientAgentFilterMiddleware {
    pub state: Arc<AppState>,
    pub config: OnceCell<Arc<UserAgent>>,
}

impl ClientAgentFilterMiddleware {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            config: OnceCell::new(),
        }
    }

    async fn is_client_allowed(&self, client: &str) -> bool {
        if client.is_empty() {
            return false;
        }

        let client_config = self.get_client_config().await;
        let client_lower = client.to_lowercase();

        match client_config.is_allow_mode() {
            true => {
                client_config.allow_ua.is_empty()
                    || client_config.allow_ua.iter().any(|rule| {
                        self.is_ua_matching(&client_lower, &rule.to_lowercase())
                    })
            }
            false => {
                client_config.deny_ua.is_empty()
                    || !client_config.deny_ua.iter().any(|rule| {
                        self.is_ua_matching(&client_lower, &rule.to_lowercase())
                    })
            }
        }
    }

    fn is_ua_matching(&self, ua: &str, rule: &str) -> bool {
        UserAgentMatcher::is_ua_matching(ua, rule)
    }

    async fn get_client_config(&self) -> Arc<UserAgent> {
        let config_arc = self
            .config
            .get_or_init(|| async {
                let config = self.state.get_config().await;
                Arc::new(config.clone().user_agent)
            })
            .await;

        config_arc.clone()
    }
}

#[async_trait]
impl Middleware for ClientAgentFilterMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(
            CLIENT_FILTER_LOGGER_DOMAIN,
            "Starting user agent filter middleware..."
        );

        let ua_lower = ctx
            .headers
            .get(HEADER_CLIENT_KEY)
            .or_else(|| ctx.headers.get(header::USER_AGENT))
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_lowercase());

        let ua = ua_lower.as_deref().unwrap_or("");
        let is_allowed = self.is_client_allowed(ua).await;

        if is_allowed {
            info_log!(CLIENT_FILTER_LOGGER_DOMAIN, "Allowed client: {}", ua);
            next(ctx, body).await
        } else {
            error_log!(CLIENT_FILTER_LOGGER_DOMAIN, "Forbidden client: {}", ua);
            ResponseBuilder::with_status_code(StatusCode::FORBIDDEN)
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}

pub struct UserAgentMatcher;

impl UserAgentMatcher {
    pub fn is_ua_matching(ua: &str, rule: &str) -> bool {
        if rule.is_empty() {
            return true;
        }
        if ua.is_empty() {
            return false;
        }

        ua.as_bytes()
            .windows(rule.len())
            .any(|window| window.eq_ignore_ascii_case(rule.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ua_matching_logic() {
        // --- Test cases for the "infuse" rule ---

        // ✅ Case 1: A standard "infuse" user agent should match.
        let result1 = UserAgentMatcher::is_ua_matching(
            "Infuse-Direct/7.8.3 (Apple TV)",
            "infuse-direct",
        );
        println!("result1: {}, expected true", result1);

        // ❌ Case 2: A "infuse-library" user agent should be denied.
        let result2 = UserAgentMatcher::is_ua_matching(
            "infuse-library/1.0",
            "infuse-direct",
        );
        println!("result2: {}, expected false", result2);

        // ❌ Case 3: A "infuse-download" user agent should be denied.
        let result3 = UserAgentMatcher::is_ua_matching(
            "infuse-download; Infuse/7.0",
            "infuse-direct",
        );
        println!("result3: {}, expected false", result3);

        // --- Test cases for the generic rule ---

        // ✅ Case 4: A simple substring match should return true.
        let chrome_ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/127.0.0.0 Safari/537.36";
        let result4 = UserAgentMatcher::is_ua_matching(chrome_ua, "Chrome");
        println!("result4: {}, expected true", result4);

        // ❌ Case 5: A non-matching substring should return false.
        let result5 = UserAgentMatcher::is_ua_matching(chrome_ua, "Firefox");
        println!("result5: {}, expected false", result5);

        // ✅ Case 6: rule empty
        let result6 = UserAgentMatcher::is_ua_matching(chrome_ua, "");
        println!("result6: {}, expected true", result6);

        // ❌ Case 7: ua empty
        let result7 = UserAgentMatcher::is_ua_matching("", "Infuse-Direct");
        println!("result7: {}, expected false", result7);
    }
}
