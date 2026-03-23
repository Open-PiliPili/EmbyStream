use hyper::Uri;
use percent_encoding::{
    AsciiSet, CONTROLS, NON_ALPHANUMERIC, utf8_percent_encode,
};
use thiserror::Error;

use crate::config::backend::{BackendNode, WebDavConfig};

use super::{
    DEFAULT_QUERY_PARAM, MODE_PATH_JOIN, MODE_QUERY_PATH, MODE_URL_TEMPLATE,
    TEMPLATE_PLACEHOLDER,
};

#[derive(Debug, Error)]
pub enum WebDavUrlError {
    #[error("empty WebDav base_url")]
    EmptyBaseUrl,
    #[error("empty logical file path")]
    EmptyLogicalPath,
    #[error("url template missing {{file_path}} placeholder")]
    MissingTemplatePlaceholder,
    #[error("WebDav config required for url_mode query_path or url_template")]
    MissingWebDavConfig,
    #[error("invalid upstream uri: {0}")]
    InvalidUri(String),
}

fn trim_slash(s: &str) -> &str {
    s.trim_matches('/')
}

/// Characters encoded inside a single path segment (space, delimiters, percent).
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b'%')
    .add(b'/')
    .add(b' ')
    .add(b'?')
    .add(b'#')
    .add(b'[')
    .add(b']');

fn encode_query_value(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn encode_path_segments(logical_path: &str) -> String {
    trim_slash(logical_path)
        .split('/')
        .filter(|p| !p.is_empty())
        .map(|seg| utf8_percent_encode(seg, PATH_SEGMENT).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn node_origin_and_prefix(
    node: &BackendNode,
) -> Result<String, WebDavUrlError> {
    if node.base_url.trim().is_empty() {
        return Err(WebDavUrlError::EmptyBaseUrl);
    }
    let base = node.uri().to_string();
    Ok(trim_slash(&base).to_string())
}

fn build_path_join(
    node: &BackendNode,
    logical_path: &str,
) -> Result<String, WebDavUrlError> {
    let origin = node_origin_and_prefix(node)?;
    let segments = encode_path_segments(logical_path);
    if segments.is_empty() {
        return Err(WebDavUrlError::EmptyLogicalPath);
    }
    Ok(format!("{origin}/{segments}"))
}

fn build_query_path(
    node: &BackendNode,
    logical_path: &str,
    cfg: &WebDavConfig,
) -> Result<String, WebDavUrlError> {
    let origin = node_origin_and_prefix(node)?;
    let param = if cfg.query_param.trim().is_empty() {
        DEFAULT_QUERY_PARAM
    } else {
        cfg.query_param.trim()
    };
    let value = encode_query_value(logical_path.trim());
    if value.is_empty() && logical_path.trim().is_empty() {
        return Err(WebDavUrlError::EmptyLogicalPath);
    }
    Ok(format!("{origin}?{param}={value}"))
}

fn placeholder_uses_path_encoding(template: &str) -> bool {
    let Some(ph_idx) = template.find(TEMPLATE_PLACEHOLDER) else {
        return true;
    };
    match template.find('?') {
        None => true,
        Some(q) => ph_idx < q,
    }
}

fn build_from_template(
    template: &str,
    logical_path: &str,
) -> Result<String, WebDavUrlError> {
    if !template.contains(TEMPLATE_PLACEHOLDER) {
        return Err(WebDavUrlError::MissingTemplatePlaceholder);
    }
    let encoded = if placeholder_uses_path_encoding(template) {
        encode_path_segments(logical_path)
    } else {
        encode_query_value(logical_path.trim())
    };
    if encoded.is_empty() && trim_slash(logical_path).is_empty() {
        return Err(WebDavUrlError::EmptyLogicalPath);
    }
    Ok(template.replace(TEMPLATE_PLACEHOLDER, &encoded))
}

fn parse_uri(s: String) -> Result<Uri, WebDavUrlError> {
    s.parse::<Uri>()
        .map_err(|_| WebDavUrlError::InvalidUri(s.clone()))
}

/// Builds the upstream HTTP(S) URI for a WebDav node from the decrypted logical path.
pub fn build_upstream_uri(
    node: &BackendNode,
    logical_path: &str,
    cfg: Option<&WebDavConfig>,
) -> Result<Uri, WebDavUrlError> {
    if trim_slash(logical_path).is_empty() {
        return Err(WebDavUrlError::EmptyLogicalPath);
    }

    let mode = cfg
        .map(|c| c.url_mode.as_str())
        .filter(|m| !m.trim().is_empty())
        .unwrap_or(MODE_PATH_JOIN);

    let raw = match mode {
        MODE_QUERY_PATH => {
            let c = cfg.ok_or(WebDavUrlError::MissingWebDavConfig)?;
            build_query_path(node, logical_path, c)?
        }
        MODE_URL_TEMPLATE => {
            let c = cfg.ok_or(WebDavUrlError::MissingWebDavConfig)?;
            if c.url_template.trim().is_empty() {
                return Err(WebDavUrlError::MissingTemplatePlaceholder);
            }
            build_from_template(c.url_template.trim(), logical_path)?
        }
        _ => build_path_join(node, logical_path)?,
    };

    parse_uri(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::backend::BackendNode;

    fn sample_node() -> BackendNode {
        BackendNode {
            name: "t".into(),
            backend_type: crate::core::backend::webdav::BACKEND_TYPE.into(),
            pattern: String::new(),
            pattern_regex: None,
            base_url: "http://127.0.0.1".into(),
            port: "5005".into(),
            path: "webdav".into(),
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

    #[test]
    fn path_join_appends_encoded_segments() {
        let node = sample_node();
        let uri =
            build_upstream_uri(&node, "/media/foo bar.mkv", None).expect("uri");
        assert_eq!(
            uri.to_string(),
            "http://127.0.0.1:5005/webdav/media/foo%20bar.mkv"
        );
    }

    #[test]
    fn path_join_rejects_empty_path() {
        let node = sample_node();
        let err = build_upstream_uri(&node, "///", None).expect_err("empty");
        assert!(matches!(err, WebDavUrlError::EmptyLogicalPath));
    }

    #[test]
    fn query_path_encodes_value() {
        let node = sample_node();
        let cfg = WebDavConfig {
            url_mode: MODE_QUERY_PATH.into(),
            query_param: "path".into(),
            ..Default::default()
        };
        let uri =
            build_upstream_uri(&node, "/media/a", Some(&cfg)).expect("uri");
        assert!(
            uri.to_string()
                .starts_with("http://127.0.0.1:5005/webdav?path=")
        );
        assert!(uri.query().is_some());
    }

    #[test]
    fn template_in_query_uses_query_encoding() {
        let node = sample_node();
        let cfg = WebDavConfig {
            url_mode: MODE_URL_TEMPLATE.into(),
            url_template:
                "http://127.0.0.1:5005/item?path={file_path}&sort=desc".into(),
            ..Default::default()
        };
        let uri =
            build_upstream_uri(&node, "/media/x", Some(&cfg)).expect("uri");
        let s = uri.to_string();
        assert!(s.contains("sort=desc"));
        assert!(!s.contains("{file_path}"));
    }

    #[test]
    fn template_missing_placeholder_errors() {
        let node = sample_node();
        let cfg = WebDavConfig {
            url_mode: MODE_URL_TEMPLATE.into(),
            url_template: "http://127.0.0.1/no-placeholder".into(),
            ..Default::default()
        };
        let err = build_upstream_uri(&node, "/a", Some(&cfg)).expect_err("err");
        assert!(matches!(err, WebDavUrlError::MissingTemplatePlaceholder));
    }

    #[test]
    fn empty_base_url_errors() {
        let mut node = sample_node();
        node.base_url.clear();
        let err = build_upstream_uri(&node, "/a", None).expect_err("err");
        assert!(matches!(err, WebDavUrlError::EmptyBaseUrl));
    }
}
