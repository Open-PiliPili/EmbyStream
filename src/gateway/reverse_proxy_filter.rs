use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode, body::Incoming};

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

        let host = ctx.get_host().unwrap_or_default();
        let is_need_anti = self.config.is_need_anti(&host);

        if !is_need_anti {
            next(ctx, body).await
        } else {
            error_log!(
                REVERSE_PROXY_FILTER_LOGGER_DOMAIN,
                "Forbidden host: {}",
                host
            );
            ResponseBuilder::with_status_code(StatusCode::FORBIDDEN)
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
