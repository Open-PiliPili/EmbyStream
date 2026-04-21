use std::{borrow::Cow, fs, path::Path};

use axum::{
    extract::{OriginalUri, State},
    http::{HeaderValue, StatusCode, header},
    response::IntoResponse,
};

use super::api::WebAppState;

pub const FRONTEND_DIST_DIR: &str = "web/dist";

include!(concat!(env!("OUT_DIR"), "/generated_web_assets.rs"));

pub async fn serve_frontend(
    State(_state): State<WebAppState>,
    OriginalUri(uri): OriginalUri,
) -> impl IntoResponse {
    let requested = normalize_request_path(uri.path());
    let is_spa_route = !requested.contains('.');

    let resolved =
        embedded_or_filesystem_asset(requested.as_ref()).or_else(|| {
            if is_spa_route {
                embedded_or_filesystem_asset("index.html")
            } else {
                None
            }
        });

    let Some((bytes, content_type)) = resolved else {
        return (
            StatusCode::NOT_FOUND,
            security_headers("text/plain; charset=utf-8", false, true),
            Cow::Borrowed("Not Found"),
        )
            .into_response();
    };

    (
        StatusCode::OK,
        security_headers(
            content_type,
            content_type.starts_with("text/html"),
            false,
        ),
        bytes,
    )
        .into_response()
}

fn security_headers(
    content_type: &'static str,
    is_html: bool,
    no_store: bool,
) -> [(header::HeaderName, HeaderValue); 6] {
    let cache_control = if no_store {
        HeaderValue::from_static("no-store")
    } else {
        HeaderValue::from_static("no-cache")
    };

    let csp = if is_html {
        HeaderValue::from_static(
            "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'unsafe-eval'; style-src 'self'; img-src 'self' https: data:; font-src 'self' data:; connect-src 'self' https://api.iconify.design https://api.unisvg.com https://api.simplesvg.com; worker-src 'self'",
        )
    } else {
        HeaderValue::from_static("default-src 'none'; frame-ancestors 'none'")
    };

    [
        (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
        (header::CACHE_CONTROL, cache_control),
        (header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")),
        (
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ),
        (
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ),
        (header::CONTENT_SECURITY_POLICY, csp),
    ]
}

fn normalize_request_path(path: &str) -> Cow<'_, str> {
    let trimmed = path.trim_start_matches('/');
    if trimmed.is_empty() {
        Cow::Borrowed("index.html")
    } else {
        Cow::Owned(trimmed.to_string())
    }
}

fn embedded_or_filesystem_asset(path: &str) -> Option<(Vec<u8>, &'static str)> {
    if let Some(asset) = embedded_asset(path) {
        return Some((asset.bytes.to_vec(), asset.content_type));
    }

    filesystem_asset(path)
}

fn filesystem_asset(path: &str) -> Option<(Vec<u8>, &'static str)> {
    let file_path = Path::new(FRONTEND_DIST_DIR).join(path);
    let bytes = fs::read(file_path).ok()?;
    Some((bytes, content_type_for(path)))
}

fn content_type_for(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("map") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
