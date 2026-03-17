use std::{fmt::Debug, time::Instant};

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::BodyExt;
use hyper::{
    Method, Response, StatusCode,
    body::Incoming,
    header::{self, HeaderName, HeaderValue},
};

use super::{
    cacheable_routes::find_cacheable_route,
    chain::{Middleware, Next},
    context::Context,
    response::{BoxBodyType, ResponseBuilder},
};
use crate::{
    API_CACHE_LOGGER_DOMAIN, REVERSE_PROXY_LOGGER_DOMAIN, cache::GeneralCache,
    debug_log, error_log, info_log, warn_log,
};

const ROOT_PATH: &str = "/";
const WEB_INDEX_REDIRECT: &str = "/web/index.html";

#[derive(Clone, Debug)]
struct CachedApiResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    cached_at: Instant,
    ttl_seconds: u64,
}

impl CachedApiResponse {
    fn is_expired(&self) -> bool {
        self.cached_at.elapsed().as_secs() > self.ttl_seconds
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
}

impl ReverseProxyMiddleware {
    pub fn new(emby_base_url: String, api_cache: GeneralCache) -> Self {
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Failed to build reqwest client for reverse proxy");

        Self {
            emby_base_url,
            http_client,
            api_cache,
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
        method: &Method,
        uri_string: &str,
        body_bytes: Option<&Bytes>,
    ) -> String {
        match body_bytes {
            Some(bytes) if !bytes.is_empty() => {
                let body_hash = format!("{:x}", md5::compute(bytes));
                format!("{}:{}:{}", method, uri_string, body_hash)
            }
            _ => format!("{}:{}", method, uri_string),
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

        info_log!(API_CACHE_LOGGER_DOMAIN, "[CACHE HIT] key={}", cache_key);
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
            cached_at: Instant::now(),
            ttl_seconds,
        };

        self.api_cache.insert(cache_key.clone(), cached);
        info_log!(
            API_CACHE_LOGGER_DOMAIN,
            "[CACHE STORE] key={}, ttl={}s, body_size={}",
            cache_key,
            ttl_seconds,
            body.len()
        );
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

        let cacheable_route =
            find_cacheable_route(&ctx.path, ctx.method.as_str());

        let body_bytes = Self::read_body(body).await;

        let cache_key = cacheable_route.map(|_| {
            let uri_string = ctx
                .uri
                .path_and_query()
                .map(|pq| pq.to_string())
                .unwrap_or_else(|| ctx.path.clone());

            Self::build_cache_key(&ctx.method, &uri_string, body_bytes.as_ref())
        });

        if let Some(ref key) = cache_key {
            if let Some(cached_response) = self.try_cache_hit(key) {
                return cached_response;
            }
        }

        let Some((status, resp_headers, resp_body)) =
            self.proxy_and_read(&ctx, body_bytes).await
        else {
            return ResponseBuilder::with_status_code(StatusCode::BAD_GATEWAY);
        };

        if let (Some(route), Some(key)) = (cacheable_route, cache_key) {
            if status.is_success() {
                self.store_cache(
                    key,
                    status,
                    &resp_headers,
                    &resp_body,
                    route.ttl_seconds,
                );
            }
        }

        Self::build_proxy_response(status, &resp_headers, resp_body)
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}
