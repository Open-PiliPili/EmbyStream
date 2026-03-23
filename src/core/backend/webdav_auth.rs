use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use dashmap::DashMap;
use hyper::{HeaderMap, StatusCode, Uri, header};

use crate::{
    WEBDAV_AUTH_LOGGER_DOMAIN,
    config::backend::{BackendNode, WebDavConfig},
    debug_log, info_log,
};

use super::upstream_proxy;

const BASIC_PREFIX: &str = "Basic ";

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

fn creds_configured(cfg: &WebDavConfig) -> bool {
    !cfg.username.trim().is_empty() || !cfg.password.trim().is_empty()
}

/// Returns `Authorization` line (`Basic …`) for upstream requests when credentials are set.
#[allow(clippy::result_unit_err)]
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

pub async fn authorization_header_for_proxy(
    cache: &DashMap<String, String>,
    node: &BackendNode,
    upstream_uri: &Uri,
    cfg: &WebDavConfig,
    client_headers: Option<&HeaderMap>,
) -> Result<Option<String>, ()> {
    if !creds_configured(cfg) {
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
    cache.insert(key, auth_line.clone());
    Ok(Some(auth_line))
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
    use super::*;

    #[test]
    fn basic_header_format() {
        let h = basic_authorization_header("user", "pass");
        assert!(h.starts_with(BASIC_PREFIX));
        let rest = h.strip_prefix(BASIC_PREFIX).expect("prefix");
        let decoded = B64.decode(rest.as_bytes()).expect("b64");
        let s = String::from_utf8(decoded).expect("utf8");
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
}
