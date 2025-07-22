use async_trait::async_trait;
use hyper::{Response, StatusCode, body::Incoming};

use super::{
    chain::{Middleware, Next},
    response::{BoxBodyType, ResponseBuilder},
};
use crate::gateway::context::Context;
use crate::{GATEWAY_LOGGER_DOMAIN, debug_log, error_log};

#[derive(Clone)]
pub struct OptionsMiddleware;

#[async_trait]
impl Middleware for OptionsMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting options middleware...");

        if ctx.method == hyper::Method::OPTIONS {
            error_log!(
                GATEWAY_LOGGER_DOMAIN,
                "OPTIONS request received, aborting with status 204"
            );
            return ResponseBuilder::with_status_code(StatusCode::NO_CONTENT);
        }

        next(ctx, body).await
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
