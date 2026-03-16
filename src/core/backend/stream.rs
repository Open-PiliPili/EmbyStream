use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Response, StatusCode, body::Incoming};

use super::{result::Result as AppStreamResult, service::StreamService};
use crate::{
    GATEWAY_LOGGER_DOMAIN, REMOTE_STREAMER_LOGGER_DOMAIN, debug_log, info_log,
    warn_log,
};
use crate::{
    config::backend::BackendNode,
    core::request::Request as AppStreamRequest,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

/// Middleware for routing requests to backend nodes
///
/// Matches incoming requests to backend nodes based on pattern matching
/// and priority, then delegates to the stream service for processing.
#[derive(Clone)]
pub struct StreamMiddleware {
    backend_nodes: Vec<BackendNode>,
    stream_service: Arc<dyn StreamService>,
}

impl StreamMiddleware {
    /// Create a new StreamMiddleware instance
    ///
    /// # Arguments
    /// * `backend_nodes` - List of backend nodes to match against (sorted by priority)
    /// * `stream_service` - Service to handle matched requests
    pub fn new(
        mut backend_nodes: Vec<BackendNode>,
        stream_service: Arc<dyn StreamService>,
    ) -> Self {
        backend_nodes.sort_by_key(|node| node.priority);
        Self {
            backend_nodes,
            stream_service,
        }
    }
}

#[async_trait]
impl Middleware for StreamMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(GATEWAY_LOGGER_DOMAIN, "Starting stream middleware...");
        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Request path: {}, method: {:?}",
            ctx.path,
            ctx.method
        );

        let request_path = ctx.path.clone();

        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Searching for matching backend node among {} nodes",
            self.backend_nodes.len()
        );

        let matched_node = self.backend_nodes.iter().find(|node| {
            if let Some(ref regex) = node.pattern_regex {
                let matches = regex.is_match(&request_path);
                debug_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "Checking node '{}': regex_pattern='{}', matches={}",
                    node.name,
                    node.pattern,
                    matches
                );
                matches
            } else if !node.pattern.is_empty() {
                let matches = request_path.starts_with(&node.pattern);
                debug_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "Checking node '{}': pattern='{}', matches={}",
                    node.name,
                    node.pattern,
                    matches
                );
                matches
            } else {
            let prefix = format!("/{}", node.path.trim_matches('/'));
            let matches = request_path.starts_with(&prefix);
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                    "Checking node '{}': path_prefix='{}', matches={}",
                node.name,
                prefix,
                matches
            );
            matches
            }
        });

        if let Some(node) = matched_node {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "✓ Matched backend node: name='{}', \
                 type='{}', proxy_mode='{}', base_url='{}', uuid='{}'",
                node.name,
                node.backend_type,
                node.proxy_mode,
                node.base_url,
                node.uuid
            );

            let host = ctx
                .headers
                .get("host")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            if node.anti_reverse_proxy.is_need_anti(host) {
                info_log!(
                    REMOTE_STREAMER_LOGGER_DOMAIN,
                    "Blocked request from host: {} for node: {}",
                    host,
                    node.name
                );
                return ResponseBuilder::with_status_code(
                    StatusCode::FORBIDDEN,
                );
            }

            let stream_request = AppStreamRequest {
                uri: ctx.uri,
                original_headers: ctx.headers,
                request_start_time: ctx.start_time,
                node: Some(node.clone()),
            };

            let result =
                self.stream_service.handle_request(stream_request).await;

            match result {
                Ok(service_result) => match service_result {
                    AppStreamResult::Stream(stream_response) => {
                        let mut response = Response::builder()
                            .status(stream_response.status)
                            .body(stream_response.body)
                            .expect("Failed to build stream response");
                        *response.headers_mut() = stream_response.headers;
                        response
                    }
                    AppStreamResult::Redirect(redirect_info) => {
                        info_log!(
                            REMOTE_STREAMER_LOGGER_DOMAIN,
                            "Redirecting backend to {:?}",
                            redirect_info.target_url
                        );
                        debug_log!(
                            REMOTE_STREAMER_LOGGER_DOMAIN,
                            "Redirecting backend headers {:?}",
                            redirect_info.final_headers.clone()
                        );
                        ResponseBuilder::with_redirect(
                            redirect_info.target_url.to_string().as_str(),
                            StatusCode::MOVED_PERMANENTLY,
                            Some(redirect_info.final_headers),
                        )
                    }
                },
                Err(status_code) => {
                    ResponseBuilder::with_status_code(status_code)
                }
            }
        } else {
            warn_log!(
                GATEWAY_LOGGER_DOMAIN,
                "No backend node matched for path: {}",
                ctx.path
            );
            next(ctx, body).await
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
