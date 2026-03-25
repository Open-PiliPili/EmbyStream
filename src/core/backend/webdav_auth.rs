use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use dashmap::DashMap;
use hyper::{HeaderMap, StatusCode, Uri, header};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    WEBDAV_AUTH_LOGGER_DOMAIN,
    config::backend::{BackendNode, WebDavConfig},
    debug_log, info_log,
};

use super::upstream_proxy;

const BASIC_PREFIX: &str = "Basic ";

/// Per-node mutex map: same key as `webdav_auth_cache` so only one probe runs at a time per node.
pub type WebDavAuthProbeLocks = DashMap<String, Arc<AsyncMutex<()>>>;

fn cache_key(node: &BackendNode) -> String {
    let base = node.base_url.trim_end_matches('/').to_lowercase();
    format!("{}|{}", node.name, base)
}

fn basic_authorization_header(username: &str, password: &str) -> String {
    let mut combined =
        String::with_capacity(username.len() + password.len() + 1);
    combined.push_str(username);
    combined.push(':');
    combined.push_str(password);
    let encoded = B64.encode(combined.as_bytes());
    let mut out = String::with_capacity(BASIC_PREFIX.len() + encoded.len());
    out.push_str(BASIC_PREFIX);
    out.push_str(&encoded);
    out
}

/// True when WebDAV Basic should be attached (non-empty username or password after trim).
pub fn credentials_configured(cfg: &WebDavConfig) -> bool {
    !cfg.username.trim().is_empty() || !cfg.password.trim().is_empty()
}

/// Resolves User-Agent for the upstream probe: client header, node config, then system default.
fn probe_user_agent(
    client_headers: Option<&HeaderMap>,
    cfg: &WebDavConfig,
) -> String {
    if let Some(h) = client_headers {
        if let Some(v) = h.get(header::USER_AGENT).and_then(|x| x.to_str().ok())
        {
            let t = v.trim();
            if !t.is_empty() {
                return t.to_string();
            }
        }
    }
    let t = cfg.user_agent.trim();
    if !t.is_empty() {
        return t.to_string();
    }
    crate::system::SystemInfo::new().get_user_agent()
}

/// Runs HEAD (or ranged GET) probe and stores `Basic` line in `cache` on success.
async fn probe_and_cache_basic_line(
    cache: &DashMap<String, String>,
    key: &str,
    node: &BackendNode,
    upstream_uri: &Uri,
    cfg: &WebDavConfig,
    client_headers: Option<&HeaderMap>,
    stream_session_id: Option<&str>,
) -> Result<String, ()> {
    let auth_line = basic_authorization_header(&cfg.username, &cfg.password);
    let ua = probe_user_agent(client_headers, cfg);

    debug_log!(
        WEBDAV_AUTH_LOGGER_DOMAIN,
        "Probing WebDav Basic auth for node='{}' uri={}",
        node.name,
        upstream_uri
    );
    let status = upstream_proxy::probe_authorization(
        upstream_uri.clone(),
        &auth_line,
        &ua,
        stream_session_id,
    )
    .await
    .map_err(|_| ())?;

    if status == StatusCode::UNAUTHORIZED {
        return Err(());
    }

    info_log!(
        WEBDAV_AUTH_LOGGER_DOMAIN,
        "WebDav Basic auth cached node='{}' probe_status={}",
        node.name,
        status
    );
    cache.insert(key.to_string(), auth_line.clone());
    Ok(auth_line)
}

/// Returns `Authorization` line (`Basic …`) for upstream requests when credentials are set.
/// When username and password are both empty, returns `Ok(None)` immediately (no probe, no lock).
#[allow(clippy::result_unit_err)]
pub async fn authorization_header_for_proxy(
    cache: &DashMap<String, String>,
    probe_locks: &WebDavAuthProbeLocks,
    node: &BackendNode,
    upstream_uri: &Uri,
    cfg: &WebDavConfig,
    client_headers: Option<&HeaderMap>,
    stream_session_id: Option<&str>,
) -> Result<Option<String>, ()> {
    if !credentials_configured(cfg) {
        return Ok(None);
    }

    let key = cache_key(node);

    if let Some(cached) = cache.get(&key) {
        debug_log!(
            WEBDAV_AUTH_LOGGER_DOMAIN,
            "Basic auth cache hit node='{}' key_prefix='{}'",
            node.name,
            key
        );
        return Ok(Some(cached.clone()));
    }

    // Single-flight: concurrent cold-start requests wait here; one probe fills the cache.
    let probe_mutex = probe_locks
        .entry(key.clone())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone();

    let _probe_guard = probe_mutex.lock().await;

    if let Some(cached) = cache.get(&key) {
        debug_log!(
            WEBDAV_AUTH_LOGGER_DOMAIN,
            "Basic auth cache hit after probe wait node='{}'",
            node.name
        );
        return Ok(Some(cached.clone()));
    }

    let line = probe_and_cache_basic_line(
        cache,
        &key,
        node,
        upstream_uri,
        cfg,
        client_headers,
        stream_session_id,
    )
    .await?;
    Ok(Some(line))
}

pub fn invalidate(cache: &DashMap<String, String>, node: &BackendNode) {
    cache.remove(&cache_key(node));
}

/// Builds a single-entry header map for `Authorization` when `line` is present.
pub fn extra_headers_from_auth_line(
    line: &str,
) -> Result<HeaderMap, &'static str> {
    let mut map = HeaderMap::new();
    let value = line
        .parse()
        .map_err(|_| "invalid authorization header value")?;
    map.insert(header::AUTHORIZATION, value);
    Ok(map)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::{Arc, Once};

    use rustls::crypto::aws_lc_rs;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    static RUSTLS_CRYPTO_INIT: Once = Once::new();

    /// `hyper_rustls` needs a process default crypto provider before any HTTPS (or pooled) use.
    fn ensure_rustls_crypto_provider() {
        RUSTLS_CRYPTO_INIT.call_once(|| {
            let _ = aws_lc_rs::default_provider().install_default();
        });
    }

    use super::*;
    use crate::config::backend::BackendNode;

    #[test]
    fn basic_header_format() {
        let h = basic_authorization_header("user", "pass");
        assert!(h.starts_with(BASIC_PREFIX));
        let rest = match h.strip_prefix(BASIC_PREFIX) {
            Some(r) => r,
            None => panic!("missing Basic prefix"),
        };
        let decoded = match B64.decode(rest.as_bytes()) {
            Ok(d) => d,
            Err(e) => panic!("b64 decode: {e}"),
        };
        let s = match String::from_utf8(decoded) {
            Ok(s) => s,
            Err(e) => panic!("utf8: {e}"),
        };
        assert_eq!(s, "user:pass");
    }

    #[test]
    fn cache_key_stable() {
        let node = BackendNode {
            name: "n1".into(),
            backend_type: String::new(),
            pattern: String::new(),
            pattern_regex: None,
            base_url: "HTTPS://EXAMPLE.COM/".into(),
            port: String::new(),
            path: String::new(),
            priority: 0,
            proxy_mode: String::new(),
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
        };
        assert_eq!(cache_key(&node), "n1|https://example.com");
    }

    #[test]
    fn credentials_configured_requires_non_empty_field() {
        let empty = WebDavConfig::default();
        assert!(!credentials_configured(&empty));

        let mut u = WebDavConfig::default();
        u.username = "a".into();
        assert!(credentials_configured(&u));

        let mut p = WebDavConfig::default();
        p.password = "b".into();
        assert!(credentials_configured(&p));
    }

    fn sample_webdav_node(base_url: &str) -> BackendNode {
        BackendNode {
            name: "probe-test-node".into(),
            backend_type: crate::core::backend::webdav::BACKEND_TYPE.into(),
            pattern: String::new(),
            pattern_regex: None,
            base_url: base_url.into(),
            port: String::new(),
            path: String::new(),
            priority: 0,
            proxy_mode: String::new(),
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

    /// Minimal HTTP/1.1 server: counts `HEAD` requests, responds `200` with empty body.
    async fn spawn_head_counter_server() -> (String, Arc<AtomicU32>) {
        let listener = match TcpListener::bind("127.0.0.1:0").await {
            Ok(l) => l,
            Err(e) => panic!("bind test listener: {e}"),
        };
        let addr = match listener.local_addr() {
            Ok(a) => a,
            Err(e) => panic!("listener addr: {e}"),
        };
        let head_hits = Arc::new(AtomicU32::new(0));
        let hits_listen = Arc::clone(&head_hits);

        tokio::spawn(async move {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(x) => x,
                    Err(_) => break,
                };
                let hits = Arc::clone(&hits_listen);
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let n = match stream.read(&mut buf).await {
                        Ok(n) => n,
                        Err(_) => return,
                    };
                    let req = String::from_utf8_lossy(&buf[..n]);
                    if req.starts_with("HEAD ") {
                        hits.fetch_add(1, Ordering::SeqCst);
                    }
                    let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                    let _ = stream.write_all(resp).await;
                });
            }
        });

        let base = format!("http://{}:{}", addr.ip(), addr.port());
        (base, head_hits)
    }

    /// Concurrent cache misses must perform only one upstream probe (single HEAD) per node key.
    #[tokio::test]
    async fn concurrent_cache_miss_single_probe() {
        ensure_rustls_crypto_provider();
        let (base, head_count) = spawn_head_counter_server().await;
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        let uri: Uri = match format!("{base}/media/file.bin").parse() {
            Ok(u) => u,
            Err(e) => panic!("test uri: {e}"),
        };
        let cache = Arc::new(DashMap::<String, String>::new());
        let locks = Arc::new(WebDavAuthProbeLocks::new());
        let node = sample_webdav_node(&base);
        let cfg = WebDavConfig {
            username: "u".into(),
            password: "p".into(),
            ..Default::default()
        };

        const N: usize = 24;
        let mut tasks = Vec::with_capacity(N);
        for _ in 0..N {
            let cache = Arc::clone(&cache);
            let locks = Arc::clone(&locks);
            let node = node.clone();
            let cfg = cfg.clone();
            let uri = uri.clone();
            tasks.push(tokio::spawn(async move {
                authorization_header_for_proxy(
                    cache.as_ref(),
                    locks.as_ref(),
                    &node,
                    &uri,
                    &cfg,
                    None,
                    None,
                )
                .await
            }));
        }

        for t in tasks {
            let joined = match t.await {
                Ok(j) => j,
                Err(e) => panic!("join probe task: {e}"),
            };
            match joined {
                Ok(Some(line)) => assert!(line.starts_with(BASIC_PREFIX)),
                other => panic!("unexpected probe outcome: {other:?}"),
            }
        }

        assert_eq!(
            head_count.load(Ordering::SeqCst),
            1,
            "single-flight: expect exactly one HEAD probe"
        );
    }
}
