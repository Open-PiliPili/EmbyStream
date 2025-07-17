use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode, body::Incoming};
use reqwest::Url;

use crate::{REVERSE_PROXY_FILTER_LOGGER_DOMAIN, debug_log, error_log};
use crate::{
    config::types::AntiReverseProxyConfig,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

#[derive(Clone)]
pub struct ReverseProxyFilterMiddleware {
    pub config: Arc<AntiReverseProxyConfig>,
}

impl ReverseProxyFilterMiddleware {
    pub fn new(config: AntiReverseProxyConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    fn is_need_anti(&self, scheme_with_host: &str) -> bool {
        if !self.config.enable {
            return false;
        }

        let input_url = match Url::parse(scheme_with_host) {
            Ok(url) => url,
            Err(_) => return false,
        };

        let trusted_url = match Url::parse(&self.config.trusted_host) {
            Ok(url) => url,
            Err(_) => return false,
        };

        match (input_url.host_str(), trusted_url.host_str()) {
            (Some(input_host), Some(trusted_host)) => !input_host.eq_ignore_ascii_case(trusted_host),
            _ => false,
        }
    }
}

#[async_trait]
impl Middleware for ReverseProxyFilterMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(
            REVERSE_PROXY_FILTER_LOGGER_DOMAIN,
            "Starting anti reverse proxy filter middleware..."
        );

        let scheme_with_host_lower = ctx.get_scheme_and_host().unwrap_or_default().to_lowercase();

        let is_need_anti = self.is_need_anti(&scheme_with_host_lower);

        if !is_need_anti {
            next(ctx, body).await
        } else {
            error_log!(
                REVERSE_PROXY_FILTER_LOGGER_DOMAIN,
                "Forbidden host: {}",
                scheme_with_host_lower
            );
            ResponseBuilder::with_status_code(StatusCode::FORBIDDEN)
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
