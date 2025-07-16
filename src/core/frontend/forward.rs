use std::sync::Arc;

use async_trait::async_trait;
use form_urlencoded;
use hyper::{Response, StatusCode, Uri, body::Incoming};
use once_cell::sync::Lazy;
use regex::Regex;

use super::service::ForwardService;
use crate::frontend::types::PathParams;
use crate::{FORWARD_LOGGER_DOMAIN, GATEWAY_LOGGER_DOMAIN, debug_log, info_log};
use crate::{
    core::request::Request as AppForwardRequest,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

static NORMAL_STREAM_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&concat!(
        r"^/(?:emby/)?videos/",    // 1. Path prefix
        r"([a-zA-Z0-9_-]+)",       // 2. Item ID capture
        r"(?:",                    // 3. Start path alternatives
        r"/(?:original|stream)",   // 4. Legacy paths
        r"(?:\.[a-zA-Z0-9]+)?",    // 5. Optional extension
        r"|/[a-zA-Z0-9_-]+\.m3u8", // 6. Direct m3u8 path
        r")$"                      // 7. Close group
    ))
    .expect("Invalid regex pattern")
});

static HLS_STREAM_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&concat!(
        r"^/(?:emby/)?videos/", // 1. Path prefix
        r"([a-zA-Z0-9_-]+)",    // 2. Item ID capture
        r"/hls\d*/",            // 3. HLS path prefix
        r"[^/]+",               // 4. Segment name
        r"(?:/\d+)?",           // 5. Optional HLS sequence
        r"\.(?:ts|m3u8)$"       // 6. HLS extensions
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
        if let Some(caps) = NORMAL_STREAM_REGEX.captures(path) {
            return caps.get(1).map(|m| m.as_str().to_owned());
        }

        HLS_STREAM_REGEX
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
    async fn handle(&self, ctx: Context, body: Option<Incoming>, next: Next) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting forward middleware...");

        let Some(item_id) = self.get_item_id(&ctx.path) else {
            return next(ctx, body).await;
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
