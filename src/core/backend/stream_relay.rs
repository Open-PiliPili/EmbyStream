//! Signed stream relay: redirect GET requests to another backend without decrypting `sign`.

use async_trait::async_trait;
use hyper::{Method, Response, StatusCode, Uri, body::Incoming, header};

use super::constants::{
    STREAM_RELAY_BACKEND_TYPE, backend_base_url_is_empty,
    backend_base_url_is_local_host,
};
use crate::{
    GATEWAY_LOGGER_DOMAIN, config::backend::BackendNode, debug_log, warn_log,
};
use crate::{
    core::sign::SignParams,
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
};

fn is_stream_relay_node(node: &BackendNode) -> bool {
    node.backend_type
        .eq_ignore_ascii_case(STREAM_RELAY_BACKEND_TYPE)
}

fn relay_nodes_sorted(mut nodes: Vec<BackendNode>) -> Vec<BackendNode> {
    nodes.retain(is_stream_relay_node);
    nodes.sort_by_key(|n| n.priority);
    nodes
}

/// Matches `request_path` against a StreamRelay node in this middleware only: the **HTTP path**
/// (e.g. `^/stream$`). Decrypted file paths are matched in [`StreamMiddleware`](crate::core::backend::stream::StreamMiddleware).
fn http_path_matches_node(request_path: &str, node: &BackendNode) -> bool {
    if let Some(re) = &node.pattern_regex {
        return re.is_match(request_path);
    }
    if !node.pattern.is_empty() {
        return request_path.starts_with(&node.pattern);
    }
    false
}

/// Full `Host` header value, lowercased (includes port when present). Compared to
/// `Uri::authority()` so `127.0.0.1:6001` → `127.0.0.1:6002` is not treated as a loop.
fn request_authority_for_loop_check(ctx: &Context) -> Option<String> {
    ctx.headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim().to_ascii_lowercase())
}

fn redirect_would_loop(
    request_authority: Option<&str>,
    request_path: &str,
    target: &Uri,
) -> bool {
    let Some(req_auth) = request_authority.filter(|h| !h.is_empty()) else {
        return false;
    };
    let Some(t_auth) = target.authority() else {
        return false;
    };
    if t_auth.as_str().to_ascii_lowercase() != req_auth {
        return false;
    }
    let t_path = target.path();
    let t_norm = if t_path.is_empty() { "/" } else { t_path };
    let r_norm = if request_path.is_empty() {
        "/"
    } else {
        request_path
    };
    t_norm == r_norm
}

fn build_redirect_location(
    node: &BackendNode,
    raw_query: Option<&str>,
) -> String {
    let mut base = node.uri().to_string();
    if base.ends_with('/') {
        base.pop();
    }
    match raw_query {
        Some(q) if !q.is_empty() => format!("{base}?{q}"),
        _ => base,
    }
}

#[derive(Clone)]
pub struct StreamRelayMiddleware {
    relay_nodes: Vec<BackendNode>,
}

impl StreamRelayMiddleware {
    pub fn new(all_nodes: Vec<BackendNode>) -> Self {
        Self {
            relay_nodes: relay_nodes_sorted(all_nodes),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.relay_nodes.is_empty()
    }
}

#[async_trait]
impl Middleware for StreamRelayMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        if self.relay_nodes.is_empty() {
            return next(ctx, body).await;
        }

        if ctx.method != Method::GET {
            return next(ctx, body).await;
        }

        let params = ctx
            .uri
            .query()
            .and_then(|q| serde_urlencoded::from_str::<SignParams>(q).ok())
            .unwrap_or_default();

        if params.sign.is_empty() {
            return next(ctx, body).await;
        }

        for node in &self.relay_nodes {
            if !http_path_matches_node(&ctx.path, node) {
                continue;
            }

            if backend_base_url_is_empty(&node.base_url)
                || backend_base_url_is_local_host(&node.base_url)
            {
                warn_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "StreamRelay node '{}': base_url is empty or loopback; skipping (forbidden)",
                    node.name
                );
                continue;
            }

            let location_str = build_redirect_location(node, ctx.uri.query());
            let target_uri: Uri = match location_str.parse() {
                Ok(u) => u,
                Err(_) => {
                    warn_log!(
                        GATEWAY_LOGGER_DOMAIN,
                        "StreamRelay node '{}': invalid redirect target {:?}",
                        node.name,
                        location_str
                    );
                    continue;
                }
            };

            let req_auth = request_authority_for_loop_check(&ctx);
            if redirect_would_loop(req_auth.as_deref(), &ctx.path, &target_uri)
            {
                warn_log!(
                    GATEWAY_LOGGER_DOMAIN,
                    "StreamRelay node '{}': skip redirect to same host/path as request (loop)",
                    node.name
                );
                continue;
            }

            debug_log!(
                GATEWAY_LOGGER_DOMAIN,
                "StreamRelay node '{}': {} -> {}",
                node.name,
                ctx.path,
                location_str
            );

            return ResponseBuilder::with_redirect(
                location_str.as_str(),
                StatusCode::MOVED_PERMANENTLY,
                None,
            );
        }

        next(ctx, body).await
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{HeaderMap, Uri};
    use regex::Regex;
    use std::time::Instant;

    use crate::config::backend::BackendNode;

    fn sample_relay_node() -> BackendNode {
        BackendNode {
            name: "relay".into(),
            backend_type: STREAM_RELAY_BACKEND_TYPE.into(),
            pattern: "^/stream$".into(),
            pattern_regex: Some(Regex::new("^/stream$").unwrap()),
            base_url: "http://198.51.100.10".into(),
            port: "60012".into(),
            path: "stream".into(),
            priority: 0,
            proxy_mode: "redirect".into(),
            client_speed_limit_kbs: 0,
            client_burst_speed_kbs: 0,
            path_rewrites: vec![],
            anti_reverse_proxy: Default::default(),
            path_rewriter_cache: vec![],
            uuid: String::new(),
            disk: None,
            open_list: None,
            direct_link: None,
            webdav: None,
        }
    }

    #[test]
    fn loop_same_authority_and_path() {
        let t: Uri = "http://127.0.0.1:60010/stream".parse().unwrap();
        assert!(redirect_would_loop(Some("127.0.0.1:60010"), "/stream", &t));
    }

    #[test]
    fn no_loop_different_port_same_ip() {
        let t: Uri = "http://127.0.0.1:60012/stream".parse().unwrap();
        assert!(!redirect_would_loop(Some("127.0.0.1:60010"), "/stream", &t));
    }

    #[test]
    fn no_loop_different_host() {
        let t: Uri = "https://c.example.com/stream".parse().unwrap();
        assert!(!redirect_would_loop(Some("b.example.com"), "/stream", &t));
    }

    #[test]
    fn build_redirect_location_preserves_query() {
        let n = sample_relay_node();
        let loc = build_redirect_location(&n, Some("sign=abc&device_id=1"));
        assert_eq!(
            loc,
            "http://198.51.100.10:60012/stream?sign=abc&device_id=1"
        );
    }

    #[test]
    fn http_path_matches_regex() {
        let n = sample_relay_node();
        assert!(http_path_matches_node("/stream", &n));
        assert!(!http_path_matches_node("/other", &n));
    }

    #[test]
    fn relay_nodes_sorted_filters_and_orders() {
        let mut a = sample_relay_node();
        a.name = "second".into();
        a.priority = 10;
        let mut b = sample_relay_node();
        b.name = "first".into();
        b.priority = 0;
        let web = BackendNode {
            name: "w".into(),
            backend_type: "WebDav".into(),
            pattern: ".*".into(),
            pattern_regex: Some(Regex::new(".*").unwrap()),
            ..sample_relay_node()
        };
        let sorted = relay_nodes_sorted(vec![web, a.clone(), b.clone()]);
        assert_eq!(sorted.len(), 2);
        assert_eq!(sorted[0].name, "first");
        assert_eq!(sorted[1].name, "second");
    }

    #[tokio::test]
    async fn middleware_301_preserves_query() {
        let mw = StreamRelayMiddleware::new(vec![sample_relay_node()]);
        let uri: Uri = "http://127.0.0.1:60010/stream?sign=dummy&device_id=x"
            .parse()
            .unwrap();
        let mut headers = hyper::HeaderMap::new();
        headers.insert(header::HOST, "127.0.0.1:60010".parse().unwrap());
        let ctx = Context::new(uri, Method::GET, headers, Instant::now());

        let next: Next = Box::new(|_ctx, _body| {
            Box::pin(async {
                ResponseBuilder::with_status_code(StatusCode::IM_A_TEAPOT)
            })
        });

        let resp = mw.handle(ctx, None, next).await;
        assert_eq!(resp.status(), StatusCode::MOVED_PERMANENTLY);
        let loc = resp
            .headers()
            .get(header::LOCATION)
            .unwrap()
            .to_str()
            .unwrap();
        assert_eq!(
            loc,
            "http://198.51.100.10:60012/stream?sign=dummy&device_id=x"
        );
    }

    #[tokio::test]
    async fn middleware_skips_when_no_sign() {
        let mw = StreamRelayMiddleware::new(vec![sample_relay_node()]);
        let uri: Uri = "http://127.0.0.1:60010/stream?foo=1".parse().unwrap();
        let ctx =
            Context::new(uri, Method::GET, HeaderMap::new(), Instant::now());
        let next: Next = Box::new(|_ctx, _body| {
            Box::pin(async {
                ResponseBuilder::with_status_code(StatusCode::IM_A_TEAPOT)
            })
        });
        let resp = mw.handle(ctx, None, next).await;
        assert_eq!(resp.status(), StatusCode::IM_A_TEAPOT);
    }
}
