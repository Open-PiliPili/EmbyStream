use std::{fmt::Debug, sync::Arc, time::Instant};

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{
    Method, Response, StatusCode,
    body::Incoming,
    header::{self, HeaderName, HeaderValue},
};

use super::{
    cacheable_routes::build_semantic_cache_key,
    cacheable_routes::find_cacheable_route,
    chain::{Middleware, Next},
    context::Context,
    response::{BoxBodyType, ResponseBuilder},
};
use crate::{
    API_CACHE_LOGGER_DOMAIN, AppState, REVERSE_PROXY_LOGGER_DOMAIN,
    cache::GeneralCache,
    client::{
        PlaybackInfoRequest, PlaybackInfoService, PlaybackInfoServiceError,
    },
    debug_log, error_log, info_log, warn_log,
};
use tokio::sync::Mutex as TokioMutex;

const ROOT_PATH: &str = "/";
const WEB_INDEX_REDIRECT: &str = "/web/index.html";

#[derive(Clone, Debug)]
struct CachedApiResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    // `GeneralCache` TTL is the upper retention bound for API entries.
    // Route freshness is enforced separately here so different routes can
    // still have their own shorter logical cache lifetime.
    stored_at: Instant,
    route_ttl_seconds: u64,
}

impl CachedApiResponse {
    fn is_expired(&self) -> bool {
        self.stored_at.elapsed().as_secs() > self.route_ttl_seconds
    }

    fn to_response(&self) -> Response<BoxBodyType> {
        let headers: Vec<(HeaderName, HeaderValue)> = self
            .headers
            .iter()
            .filter_map(|(name, value)| {
                let header_name = name.parse::<HeaderName>().ok()?;
                let header_value = HeaderValue::from_str(value).ok()?;
                Some((header_name, header_value))
            })
            .collect();

        let status =
            StatusCode::from_u16(self.status).unwrap_or(StatusCode::OK);

        ResponseBuilder::with_bytes(
            status,
            headers,
            Bytes::from(self.body.clone()),
        )
    }
}

#[derive(Clone)]
pub struct ReverseProxyMiddleware {
    emby_base_url: String,
    http_client: reqwest::Client,
    api_cache: GeneralCache,
    state: Arc<AppState>,
    playback_info_service: PlaybackInfoService,
}

impl ReverseProxyMiddleware {
    pub fn new(
        emby_base_url: String,
        api_cache: GeneralCache,
        state: std::sync::Arc<AppState>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to build reqwest client for reverse proxy");

        Self {
            emby_base_url,
            http_client,
            api_cache,
            state: state.clone(),
            playback_info_service: PlaybackInfoService::new(state),
        }
    }

    async fn read_body(body: Option<Incoming>) -> Option<Bytes> {
        let incoming = body?;
        match incoming.collect().await {
            Ok(collected) => Some(collected.to_bytes()),
            Err(e) => {
                warn_log!(
                    REVERSE_PROXY_LOGGER_DOMAIN,
                    "Failed to read request body: {:?}",
                    e
                );
                None
            }
        }
    }

    fn build_cache_key(
        ctx: &Context,
        route: &super::cacheable_routes::CompiledCacheableRoute,
        body_bytes: Option<&Bytes>,
    ) -> String {
        let semantic_key = build_semantic_cache_key(
            route,
            ctx.method.as_str(),
            &ctx.path,
            ctx.uri.query(),
        );

        match body_bytes {
            Some(bytes) if !bytes.is_empty() => {
                let body_hash = format!("{:x}", md5::compute(bytes));
                format!("{semantic_key}:{body_hash}")
            }
            _ => semantic_key,
        }
    }

    fn try_cache_hit(&self, cache_key: &str) -> Option<Response<BoxBodyType>> {
        let cached: CachedApiResponse = self.api_cache.get(cache_key)?;

        if cached.is_expired() {
            self.api_cache.remove(cache_key);
            debug_log!(
                API_CACHE_LOGGER_DOMAIN,
                "[CACHE EXPIRED] key={}",
                cache_key
            );
            return None;
        }

        info_log!(
            API_CACHE_LOGGER_DOMAIN,
            "[CACHE HIT] key={}{}",
            cache_key,
            Self::cache_key_log_suffix(cache_key)
        );
        Some(cached.to_response())
    }

    fn store_cache(
        &self,
        cache_key: String,
        status: StatusCode,
        headers: &reqwest::header::HeaderMap,
        body: &Bytes,
        ttl_seconds: u64,
    ) {
        let header_pairs: Vec<(String, String)> = headers
            .iter()
            .filter_map(|(name, value)| {
                value
                    .to_str()
                    .ok()
                    .map(|v| (name.as_str().to_owned(), v.to_owned()))
            })
            .collect();

        let cached = CachedApiResponse {
            status: status.as_u16(),
            headers: header_pairs,
            body: body.to_vec(),
            stored_at: Instant::now(),
            route_ttl_seconds: ttl_seconds,
        };

        self.api_cache.insert(cache_key.clone(), cached);
        info_log!(
            API_CACHE_LOGGER_DOMAIN,
            "[CACHE STORE] key={}, ttl={}s, body_size={}{}",
            cache_key,
            ttl_seconds,
            body.len(),
            Self::cache_key_log_suffix(&cache_key)
        );
    }

    fn should_cache_response(
        status: StatusCode,
        headers: &reqwest::header::HeaderMap,
    ) -> bool {
        status.is_success() && Self::is_json_content_type(headers)
    }

    fn is_json_content_type(headers: &reqwest::header::HeaderMap) -> bool {
        headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(|value| value.trim())
            .is_some_and(|value| value.eq_ignore_ascii_case("application/json"))
    }

    async fn proxy_to_emby(
        &self,
        ctx: &Context,
        body_bytes: Option<Bytes>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let target_url = format!(
            "{}{}",
            self.emby_base_url,
            ctx.uri
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or(ctx.path.as_str())
        );

        debug_log!(
            REVERSE_PROXY_LOGGER_DOMAIN,
            "Proxying {} {} -> {}",
            ctx.method,
            ctx.path,
            target_url
        );

        let method =
            reqwest::Method::from_bytes(ctx.method.as_str().as_bytes())
                .unwrap_or(reqwest::Method::GET);

        let mut request_builder = self.http_client.request(method, &target_url);

        for (name, value) in ctx.headers.iter() {
            if name == header::HOST || name == header::TRANSFER_ENCODING {
                continue;
            }
            if let Ok(value_str) = value.to_str() {
                request_builder =
                    request_builder.header(name.as_str(), value_str);
            }
        }

        if let Some(bytes) = body_bytes {
            if !bytes.is_empty() {
                request_builder = request_builder.body(bytes);
            }
        }

        request_builder.send().await
    }

    fn build_proxy_response(
        status: StatusCode,
        headers: &reqwest::header::HeaderMap,
        body_bytes: Bytes,
    ) -> Response<BoxBodyType> {
        let response_headers: Vec<(HeaderName, HeaderValue)> = headers
            .iter()
            .filter_map(|(name, value)| {
                if name == header::TRANSFER_ENCODING {
                    return None;
                }
                let hn =
                    HeaderName::from_bytes(name.as_str().as_bytes()).ok()?;
                let hv = HeaderValue::from_bytes(value.as_bytes()).ok()?;
                Some((hn, hv))
            })
            .collect();

        ResponseBuilder::with_bytes(status, response_headers, body_bytes)
    }

    async fn proxy_and_read(
        &self,
        ctx: &Context,
        body_bytes: Option<Bytes>,
    ) -> Option<(StatusCode, reqwest::header::HeaderMap, Bytes)> {
        let emby_response = match self.proxy_to_emby(ctx, body_bytes).await {
            Ok(resp) => resp,
            Err(e) => {
                error_log!(
                    REVERSE_PROXY_LOGGER_DOMAIN,
                    "Failed to proxy to Emby: {:?}",
                    e
                );
                return None;
            }
        };

        let status = StatusCode::from_u16(emby_response.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        let resp_headers = emby_response.headers().clone();

        match emby_response.bytes().await {
            Ok(resp_body) => Some((status, resp_headers, resp_body)),
            Err(e) => {
                error_log!(
                    REVERSE_PROXY_LOGGER_DOMAIN,
                    "Failed to read Emby response body: {:?}",
                    e
                );
                None
            }
        }
    }

    async fn lock_api_request(&self, cache_key: &str) -> Arc<TokioMutex<()>> {
        AppState::request_lock(&self.state.api_request_locks, cache_key)
    }

    fn cache_key_log_suffix(cache_key: &str) -> String {
        if let Some(series_id) =
            cache_key.strip_prefix("api:shows_nextup:method:get:series_id:")
        {
            return format!(" route=shows_nextup series_id={series_id}");
        }

        if let Some(show_id) =
            cache_key.strip_prefix("api:shows_episodes:method:get:show_id:")
        {
            return format!(" route=shows_episodes show_id={show_id}");
        }

        if let Some(rest) =
            cache_key.strip_prefix("api:user_item:method:get:user_id:")
        {
            if let Some((user_id, item_id)) = rest.split_once(":item_id:") {
                return format!(
                    " route=user_item user_id={user_id} item_id={item_id}"
                );
            }
        }

        String::new()
    }

    fn playback_info_request(
        &self,
        ctx: &Context,
    ) -> Option<PlaybackInfoRequest> {
        if ctx.method != Method::GET && ctx.method != Method::POST {
            return None;
        }

        PlaybackInfoRequest::from_http_parts(&ctx.path, ctx.uri.query()).ok()
    }

    async fn handle_playback_info_request(
        &self,
        ctx: &Context,
        request: PlaybackInfoRequest,
        body: Option<Incoming>,
    ) -> Response<BoxBodyType> {
        let _ = Self::read_body(body).await;
        let api_token = PlaybackInfoService::api_token_from_headers_and_query(
            &ctx.headers,
            ctx.uri.query(),
        );

        let playback_info = match self
            .playback_info_service
            .get(&request, api_token.as_deref())
            .await
        {
            Ok(playback_info) => playback_info,
            Err(error) => {
                let status = match error {
                    PlaybackInfoServiceError::InvalidItemId
                    | PlaybackInfoServiceError::InvalidMediaSourceId
                    | PlaybackInfoServiceError::EmptyApiToken => {
                        StatusCode::BAD_REQUEST
                    }
                    PlaybackInfoServiceError::Upstream(_) => {
                        StatusCode::BAD_GATEWAY
                    }
                };
                error_log!(
                    REVERSE_PROXY_LOGGER_DOMAIN,
                    "playback_info_request_failed method={} path={} status={} \
                     error={}",
                    ctx.method,
                    ctx.path,
                    status.as_u16(),
                    error
                );
                return ResponseBuilder::with_status_code(status);
            }
        };

        match serde_json::to_string(&playback_info) {
            Ok(body_json) => {
                ResponseBuilder::with_json(StatusCode::OK, &body_json)
            }
            Err(error) => {
                error_log!(
                    REVERSE_PROXY_LOGGER_DOMAIN,
                    "Failed to serialize playback info response: {}",
                    error
                );
                ResponseBuilder::with_status_code(
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            }
        }
    }
}

#[async_trait]
impl Middleware for ReverseProxyMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        _next: Next,
    ) -> Response<BoxBodyType> {
        debug_log!(
            REVERSE_PROXY_LOGGER_DOMAIN,
            "Starting reverse proxy middleware for {} {}",
            ctx.method,
            ctx.path
        );

        if ctx.path == ROOT_PATH {
            return ResponseBuilder::with_redirect(
                WEB_INDEX_REDIRECT,
                StatusCode::FOUND,
                None,
            );
        }

        if let Some(request) = self.playback_info_request(&ctx) {
            return self
                .handle_playback_info_request(&ctx, request, body)
                .await;
        }

        let cacheable_route =
            find_cacheable_route(&ctx.path, ctx.method.as_str());

        let body_bytes = Self::read_body(body).await;

        let cache_key = cacheable_route.map(|route| {
            Self::build_cache_key(&ctx, route, body_bytes.as_ref())
        });

        if let (Some(route), Some(key)) = (cacheable_route, cache_key) {
            if let Some(cached_response) = self.try_cache_hit(&key) {
                return cached_response;
            }

            let lock = self.lock_api_request(&key).await;
            let response = {
                let _guard = lock.lock().await;

                if let Some(cached_response) = self.try_cache_hit(&key) {
                    info_log!(
                        API_CACHE_LOGGER_DOMAIN,
                        "[CACHE WAIT HIT] key={}{}",
                        key,
                        Self::cache_key_log_suffix(&key)
                    );
                    cached_response
                } else {
                    match self.proxy_and_read(&ctx, body_bytes).await {
                        Some((status, resp_headers, resp_body)) => {
                            if Self::should_cache_response(
                                status,
                                &resp_headers,
                            ) {
                                self.store_cache(
                                    key.clone(),
                                    status,
                                    &resp_headers,
                                    &resp_body,
                                    route.ttl_seconds,
                                );
                            }

                            Self::build_proxy_response(
                                status,
                                &resp_headers,
                                resp_body,
                            )
                        }
                        None => ResponseBuilder::with_status_code(
                            StatusCode::BAD_GATEWAY,
                        ),
                    }
                }
            };

            AppState::cleanup_request_lock(
                &self.state.api_request_locks,
                &key,
                &lock,
            );

            return response;
        }

        let Some((status, resp_headers, resp_body)) =
            self.proxy_and_read(&ctx, body_bytes).await
        else {
            return ResponseBuilder::with_status_code(StatusCode::BAD_GATEWAY);
        };

        Self::build_proxy_response(status, &resp_headers, resp_body)
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use hyper::StatusCode;
    use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};

    use super::ReverseProxyMiddleware;

    #[test]
    fn should_cache_json_success_response() {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        );

        assert!(ReverseProxyMiddleware::should_cache_response(
            StatusCode::OK,
            &headers
        ));
    }

    #[test]
    fn should_not_cache_non_json_success_response() {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));

        assert!(!ReverseProxyMiddleware::should_cache_response(
            StatusCode::OK,
            &headers
        ));
    }

    #[test]
    fn should_not_cache_json_error_response() {
        let mut headers = HeaderMap::new();
        headers
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        assert!(!ReverseProxyMiddleware::should_cache_response(
            StatusCode::BAD_REQUEST,
            &headers
        ));
    }
}
