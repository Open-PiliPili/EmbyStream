use std::sync::Arc;

use async_trait::async_trait;
use form_urlencoded;
use hyper::{Response, StatusCode, Uri};
use once_cell::sync::Lazy;
use regex::Regex;

use super::service::ForwardService;
use crate::frontened::types::PathParams;
use crate::{FORWARD_LOGGER_DOMAIN, GATEWAY_LOGGER_DOMAIN, debug_log, info_log};
use crate::{
    core::request::Request as AppForwardRequest,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

static ITEM_ID_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&concat!(
        r"^/(?:emby/)?videos/",  // 1. Path prefix
        r"([\w-]+)",             // 2. Item ID capture
        r"(?:",                  // 3. Start path alternatives
        r"/(?:original|stream)", // 4. Legacy paths
        r"(?:\.[\w]+)?",         // 5. Optional extension
        r"|/hls\d*/[^/]+",       // 6. HLS path prefix
        r"(?:/\d+)?",            // 7. Optional HLS sequence
        r"\.(?:ts|m3u8)",        // 8. HLS extensions
        r")$"                    // 9. Close group
    ))
    .expect("Invalid regex pattern")
});

#[derive(Clone)]
pub struct ForwardMiddleware {
    forward_service: Arc<dyn ForwardService>,
}

impl ForwardMiddleware {
    pub fn new(forward_service: Arc<dyn ForwardService>) -> Self {
        Self { forward_service }
    }

    fn get_item_id(&self, path: &str) -> Option<String> {
        ITEM_ID_REGEX
            .captures(path)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str().to_owned())
    }

    fn get_media_source_id(&self, uri: &Uri) -> String {
        uri.query()
            .and_then(|q| {
                form_urlencoded::parse(q.as_bytes())
                    .find(|(k, _)| k.eq_ignore_ascii_case("MediaSourceId"))
                    .map(|(_, v)| v.into_owned())
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl Middleware for ForwardMiddleware {
    async fn handle<'a>(&self, ctx: Context, next: Next<'a>) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting forward middleware...");

        let Some(item_id) = self.get_item_id(&ctx.path) else {
            return next.run(ctx).await;
        };

        let path_params = PathParams {
            item_id,
            media_source_id: self.get_media_source_id(&ctx.uri),
        };

        let forward_request = AppForwardRequest {
            uri: ctx.uri,
            original_headers: ctx.headers,
            request_start_time: ctx.start_time,
        };

        let result = self
            .forward_service
            .handle_request(forward_request, path_params)
            .await;

        match result {
            Ok(redirect_info) => {
                info_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Redirecting forward to {:?}",
                    redirect_info.target_url
                );
                debug_log!(
                    FORWARD_LOGGER_DOMAIN,
                    "Redirecting forward headers {:?}",
                    redirect_info.final_headers.clone()
                );
                ResponseBuilder::with_redirect(
                    redirect_info.target_url.to_string(),
                    StatusCode::FOUND,
                    Some(redirect_info.final_headers),
                )
            }
            Err(status_code) => ResponseBuilder::with_status_code(status_code),
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
