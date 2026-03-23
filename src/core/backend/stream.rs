use std::sync::Arc;

use async_trait::async_trait;
use hyper::{Method, Response, StatusCode, Uri, body::Incoming, header};

use super::{
    constants::STREAM_RELAY_BACKEND_TYPE, result::Result as AppStreamResult,
    service::StreamService,
};
use crate::{
    AppState, GATEWAY_LOGGER_DOMAIN, REMOTE_STREAMER_LOGGER_DOMAIN, debug_log,
    error_log, info_log, warn_log,
};
use crate::{
    config::backend::BackendNode,
    core::{
        request::Request as AppStreamRequest, sign_decryptor::SignDecryptor,
    },
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
    sign::SignParams,
    util::UriExt,
};

#[derive(Clone)]
pub struct StreamMiddleware {
    backend_nodes: Vec<BackendNode>,
    stream_service: Arc<dyn StreamService>,
    state: Arc<AppState>,
}

impl StreamMiddleware {
    pub fn new(
        mut backend_nodes: Vec<BackendNode>,
        stream_service: Arc<dyn StreamService>,
        state: Arc<AppState>,
    ) -> Self {
        backend_nodes.sort_by_key(|node| node.priority);
        Self {
            backend_nodes,
            stream_service,
            state,
        }
    }

    fn find_matching_node<'a>(
        nodes: &'a [BackendNode],
        file_path: &str,
    ) -> Option<&'a BackendNode> {
        nodes.iter().find(|node| {
            if node.backend_type.eq_ignore_ascii_case(STREAM_RELAY_BACKEND_TYPE) {
                return false;
            }
            if let Some(ref regex) = node.pattern_regex {
                let matches = regex.is_match(file_path);
                debug_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "Checking node '{}': regex_pattern='{}', file_path='{}', matches={}",
                    node.name,
                    node.pattern,
                    file_path,
                    matches
                );
                matches
            } else if !node.pattern.is_empty() {
                let matches = file_path.starts_with(&node.pattern);
                debug_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "Checking node '{}': pattern='{}', file_path='{}', matches={}",
                    node.name,
                    node.pattern,
                    file_path,
                    matches
                );
                matches
            } else if !node.path.is_empty() {
                let prefix = format!("/{}", node.path.trim_matches('/'));
                let matches = file_path.starts_with(&prefix);
                debug_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "Checking node '{}': path_prefix='{}', file_path='{}', matches={}",
                    node.name,
                    prefix,
                    file_path,
                    matches
                );
                matches
            } else {
                debug_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "Node '{}' has no pattern — matches all paths as fallback",
                    node.name
                );
                true
            }
        })
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

        let params = ctx
            .uri
            .query()
            .and_then(|query| {
                serde_urlencoded::from_str::<SignParams>(query).ok()
            })
            .unwrap_or_default();

        if params.sign.is_empty() {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "No sign parameter found, passing to next middleware"
            );
            return next(ctx, body).await;
        }

        if ctx.method != Method::GET {
            warn_log!(
                GATEWAY_LOGGER_DOMAIN,
                "Signed stream rejected method {:?} (only GET allowed)",
                ctx.method,
            );
            return ResponseBuilder::with_status_code(
                StatusCode::METHOD_NOT_ALLOWED,
            );
        }

        let sign =
            match SignDecryptor::decrypt(&params.sign, &params, &self.state)
                .await
            {
                Ok(sign) => sign,
                Err(e) => {
                    error_log!(
                        GATEWAY_LOGGER_DOMAIN,
                        "Failed to decrypt sign: {:?}",
                        e
                    );
                    return ResponseBuilder::with_status_code(
                        StatusCode::BAD_REQUEST,
                    );
                }
            };

        if !sign.is_valid() {
            error_log!(GATEWAY_LOGGER_DOMAIN, "Sign is expired or invalid");
            return ResponseBuilder::with_status_code(StatusCode::GONE);
        }

        let sign_uri = match &sign.uri {
            Some(uri) => uri.clone(),
            None => {
                error_log!(GATEWAY_LOGGER_DOMAIN, "Sign has no URI");
                return ResponseBuilder::with_status_code(
                    StatusCode::BAD_REQUEST,
                );
            }
        };

        let file_path = Uri::to_path_or_url_string(&sign_uri);
        debug_log!(
            GATEWAY_LOGGER_DOMAIN,
            "Decrypted file path: '{}', searching among {} nodes",
            file_path,
            self.backend_nodes.len()
        );

        let matched_node =
            Self::find_matching_node(&self.backend_nodes, &file_path);

        if let Some(node) = matched_node {
            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "Matched backend node: name='{}', type='{}', proxy_mode='{}', uuid='{}'",
                node.name,
                node.backend_type,
                node.proxy_mode,
                node.uuid
            );

            let host = ctx
                .headers
                .get(header::HOST)
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
                sign: Some(sign),
            };

            let result =
                self.stream_service.handle_request(stream_request).await;

            match result {
                Ok(service_result) => match service_result {
                    AppStreamResult::Stream(stream_response) => {
                        match Response::builder()
                            .status(stream_response.status)
                            .body(stream_response.body)
                        {
                            Ok(mut response) => {
                                *response.headers_mut() =
                                    stream_response.headers;
                                response
                            }
                            Err(_) => ResponseBuilder::with_status_code(
                                StatusCode::INTERNAL_SERVER_ERROR,
                            ),
                        }
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
                "No backend node matched for file path: '{}'",
                file_path
            );
            next(ctx, body).await
        }
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
