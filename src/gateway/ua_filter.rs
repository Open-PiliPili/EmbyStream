use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode, header};
use tokio::sync::OnceCell;

use crate::{AppState, USER_AGENT_FILTER, error_log};
use crate::{
    config::general::UserAgent,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

#[derive(Clone)]
pub struct UserAgentFilterMiddleware {
    pub state: Arc<AppState>,
    pub config: OnceCell<Arc<UserAgent>>,
}

impl UserAgentFilterMiddleware {
    pub fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            config: OnceCell::new(),
        }
    }

    async fn is_ua_allowed(&self, ua: &str) -> bool {
        if ua.is_empty() {
            return false;
        }

        let ua_config = self.get_ua_config().await;
        let ua_lower = ua.to_lowercase();

        match ua_config.is_allow_mode() {
            true => {
                ua_config.allow_ua.is_empty()
                    || ua_config
                        .allow_ua
                        .iter()
                        .any(|rule| self.is_ua_matching(&ua_lower, &rule.to_lowercase()))
            }
            false => {
                ua_config.deny_ua.is_empty()
                    || !ua_config
                        .deny_ua
                        .iter()
                        .any(|rule| self.is_ua_matching(&ua_lower, &rule.to_lowercase()))
            }
        }
    }

    fn is_ua_matching(&self, ua: &str, rule: &str) -> bool {
        match rule {
            "infuse" => {
                ua.contains("infuse")
                    && !ua.contains("infuse-library")
                    && !ua.contains("infuse-download")
            }
            _ => ua.contains(rule),
        }
    }

    async fn get_ua_config(&self) -> Arc<UserAgent> {
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
impl Middleware for UserAgentFilterMiddleware {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        let ua_lower = ctx
            .headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_lowercase());

        let ua = ua_lower.as_deref().unwrap_or("");
        let is_allowed = self.is_ua_allowed(&ua).await;

        if is_allowed {
            next.run(ctx).await
        } else {
            error_log!(USER_AGENT_FILTER, "Forbidden user-agent: {}", ua);
            ResponseBuilder::with_status_code(StatusCode::FORBIDDEN)
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
