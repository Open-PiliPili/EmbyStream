use async_trait::async_trait;
use hyper::{Response, body::Incoming, header};

use super::{
    cacheable_routes::find_cacheable_route,
    chain::{Middleware, Next},
    context::Context,
    response::BoxBodyType,
};
use crate::{GATEWAY_LOGGER_DOMAIN, debug_log, info_log};

const SIGN_QUERY_KEY: &str = "sign";
const SLOW_REQUEST_THRESHOLD_MS: u128 = 1000;
#[derive(Clone)]
pub struct LoggerMiddleware;

fn should_log_request_context_info(ctx: &Context) -> bool {
    find_cacheable_route(&ctx.path, ctx.method.as_str()).is_some()
        || ctx
            .get_query_params()
            .as_ref()
            .is_some_and(|query| query.contains_key(SIGN_QUERY_KEY))
}

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
        let should_log_context_info = should_log_request_context_info(&ctx);

        if should_log_context_info {
            info_log!(
                GATEWAY_LOGGER_DOMAIN,
                "request_context request_id={} method={} path={}",
                request_id,
                ctx.method,
                ctx.path
            );
        } else {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "request_context request_id={} method={} path={}",
                request_id,
                ctx.method,
                ctx.path
            );
        }
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
        let should_log_info = should_log_context_info
            || is_slow
            || status.is_client_error()
            || status.is_server_error();

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

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use hyper::{HeaderMap, Method, Uri};

    use super::should_log_request_context_info;
    use crate::gateway::context::Context;

    fn build_context(method: Method, uri: &str) -> Context {
        let parsed_uri: Uri = uri.parse().expect("valid test uri");
        Context::new(
            parsed_uri,
            method,
            HeaderMap::new(),
            Instant::now(),
            "request:id:test".to_string(),
        )
    }

    #[test]
    fn cacheable_api_uses_info_logging() {
        let ctx = build_context(
            Method::GET,
            "http://localhost/emby/Shows/NextUp?SeriesId=123",
        );

        assert!(should_log_request_context_info(&ctx));
    }

    #[test]
    fn signed_stream_request_uses_info_logging() {
        let ctx = build_context(
            Method::GET,
            "http://localhost/stream?sign=abc&playback_session_id=playback:session:test",
        );

        assert!(should_log_request_context_info(&ctx));
    }

    #[test]
    fn static_web_asset_stays_debug_only() {
        let ctx =
            build_context(Method::GET, "http://localhost/emby/web/index.html");

        assert!(!should_log_request_context_info(&ctx));
    }
}
