use async_trait::async_trait;
use hyper::{Response, body::Incoming, header};

use super::{
    chain::{Middleware, Next},
    context::Context,
    response::BoxBodyType,
};
use crate::{GATEWAY_LOGGER_DOMAIN, debug_log, info_log};

const SLOW_REQUEST_THRESHOLD_MS: u128 = 1000;
#[derive(Clone)]
pub struct LoggerMiddleware;

#[async_trait]
impl Middleware for LoggerMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        let request_id = ctx.request_id.clone();
        let start_time = ctx.start_time;

        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "request_context request_id={} method={} path={}",
            request_id,
            ctx.method,
            ctx.path
        );
        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "request_details request_id={} host={:?} query={:?}",
            request_id,
            ctx.get_host(),
            ctx.get_query_params()
        );
        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "request_headers request_id={} headers={:?}",
            request_id,
            ctx.headers
        );

        if ctx.headers.contains_key(header::CONTENT_LENGTH) {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "request_body request_id={} has_body=true",
                request_id
            );
        }

        let response = next(ctx, body).await;

        let elapsed_ms = start_time.elapsed().as_millis();
        let status = response.status();
        let is_slow = elapsed_ms >= SLOW_REQUEST_THRESHOLD_MS;
        let should_log_info =
            is_slow || status.is_client_error() || status.is_server_error();

        if should_log_info {
            info_log!(
                GATEWAY_LOGGER_DOMAIN,
                "request_complete request_id={} status={} elapsed_ms={} slow={}",
                request_id,
                status.as_u16(),
                elapsed_ms,
                is_slow
            );
        } else {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "request_complete request_id={} status={} elapsed_ms={}",
                request_id,
                status.as_u16(),
                elapsed_ms
            );
        }

        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "response_headers request_id={} headers={:?}",
            request_id,
            response.headers()
        );

        response
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
