use std::sync::OnceLock;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{
    HeaderMap, Request, Response, StatusCode, Uri, body::Incoming, header,
};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};

use crate::{
    UPSTREAM_PROXY_LOGGER_DOMAIN, debug_log,
    gateway::{error::Error as GatewayError, response::BoxBodyType},
};

type HttpConnector = hyper_util::client::legacy::connect::HttpConnector;

type UpstreamConnector = hyper_rustls::HttpsConnector<HttpConnector>;

type UpstreamClient = Client<UpstreamConnector, Full<Bytes>>;

static UPSTREAM_CLIENT: OnceLock<Result<UpstreamClient, String>> =
    OnceLock::new();

const HOP_BY_HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "proxy-connection",
];

fn hop_by_hop(name: &str) -> bool {
    HOP_BY_HOP_HEADERS
        .iter()
        .any(|h| name.eq_ignore_ascii_case(h))
}

fn build_upstream_client() -> Result<UpstreamClient, String> {
    let connector = HttpsConnectorBuilder::new()
        .with_native_roots()
        .map_err(|e| format!("https native roots: {e}"))?
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Client::builder(TokioExecutor::new()).build(connector))
}

fn shared_client() -> Result<&'static UpstreamClient, &'static str> {
    let cell = UPSTREAM_CLIENT.get_or_init(build_upstream_client);
    cell.as_ref().map_err(String::as_str)
}

fn parse_header_value(raw: &str) -> Result<header::HeaderValue, GatewayError> {
    raw.parse().map_err(|_| {
        GatewayError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid HTTP header value",
        ))
    })
}

/// Copies client headers (minus hop-by-hop), applies `extra` (e.g. WebDav
/// `Authorization`, last writer wins on name clashes), strips `Host`, then sets
/// `User-Agent` to `user_agent` (for WebDav: client UA, then optional node UA,
/// then built-in default; see `resolve_upstream_user_agent`).
fn merge_request_headers(
    source: &HeaderMap,
    target: &mut HeaderMap,
    user_agent: &str,
    extra: Option<&HeaderMap>,
) -> Result<(), GatewayError> {
    for (name, value) in source.iter() {
        if hop_by_hop(name.as_str()) {
            continue;
        }
        target.append(name, value.clone());
    }

    if let Some(extra_map) = extra {
        for (name, value) in extra_map.iter() {
            if hop_by_hop(name.as_str()) {
                continue;
            }
            target.insert(name, value.clone());
        }
    }

    target.remove(header::HOST);
    target.insert(header::USER_AGENT, parse_header_value(user_agent)?);

    Ok(())
}

/// Copies upstream response metadata and pipes the body into `BoxBodyType`.
pub fn map_upstream_to_stream_response(
    upstream: Response<Incoming>,
) -> Result<(StatusCode, HeaderMap, BoxBodyType), GatewayError> {
    let (parts, body) = upstream.into_parts();
    let status = parts.status;
    let mut out_headers = HeaderMap::new();

    for (name, value) in parts.headers.iter() {
        if hop_by_hop(name.as_str()) {
            continue;
        }
        out_headers.append(name, value.clone());
    }

    let boxed: BoxBodyType = body.map_err(GatewayError::from).boxed();
    Ok((status, out_headers, boxed))
}

/// Sends a GET (streaming) request to `uri` and returns the upstream response for piping.
pub async fn forward_get(
    uri: Uri,
    client_headers: &HeaderMap,
    user_agent: &str,
    extra_upstream_headers: Option<&HeaderMap>,
) -> Result<Response<Incoming>, GatewayError> {
    let client = shared_client()
        .map_err(|msg| GatewayError::IoError(std::io::Error::other(msg)))?;

    let mut headers = HeaderMap::new();
    merge_request_headers(
        client_headers,
        &mut headers,
        user_agent,
        extra_upstream_headers,
    )?;

    let uri_for_log = uri.to_string();
    let mut req = Request::get(uri)
        .body(Full::default())
        .map_err(GatewayError::from)?;
    *req.headers_mut() = headers;

    let resp = client.request(req).await?;
    debug_log!(
        UPSTREAM_PROXY_LOGGER_DOMAIN,
        "Upstream GET {} -> {}",
        uri_for_log,
        resp.status()
    );
    Ok(resp)
}

/// Lightweight probe: HEAD, or GET with `Range: bytes=0-0` if HEAD is not allowed.
pub async fn probe_authorization(
    uri: Uri,
    authorization: &str,
    user_agent: &str,
) -> Result<StatusCode, GatewayError> {
    let client = shared_client()
        .map_err(|msg| GatewayError::IoError(std::io::Error::other(msg)))?;

    let mut head_headers = HeaderMap::new();
    head_headers
        .insert(header::AUTHORIZATION, parse_header_value(authorization)?);
    head_headers.insert(header::USER_AGENT, parse_header_value(user_agent)?);

    let mut head_req = Request::head(uri.clone())
        .body(Full::default())
        .map_err(GatewayError::from)?;
    *head_req.headers_mut() = head_headers.clone();

    let head_resp = client.request(head_req).await?;
    let (head_meta, head_body) = head_resp.into_parts();
    let _ = BodyExt::collect(head_body).await;
    let status = head_meta.status;

    if status == StatusCode::METHOD_NOT_ALLOWED
        || status == StatusCode::NOT_IMPLEMENTED
    {
        let mut get_headers = head_headers;
        get_headers.insert(
            header::RANGE,
            header::HeaderValue::from_static("bytes=0-0"),
        );
        let mut get_req = Request::get(uri)
            .body(Full::default())
            .map_err(GatewayError::from)?;
        *get_req.headers_mut() = get_headers;
        let get_resp = client.request(get_req).await?;
        let (get_meta, get_body) = get_resp.into_parts();
        let _ = BodyExt::collect(get_body).await;
        return Ok(get_meta.status);
    }

    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hop_by_hop_detects_connection() {
        assert!(hop_by_hop("Connection"));
        assert!(hop_by_hop("TRANSFER-ENCODING"));
        assert!(!hop_by_hop("range"));
        assert!(!hop_by_hop("content-type"));
    }
}
