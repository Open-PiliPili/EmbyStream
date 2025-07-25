use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::{Response, StatusCode, body::Incoming, header};
use lazy_static::lazy_static;
use regex::Regex;
use tokio::time::sleep;

use crate::{AppState, STREAM_LOGGER_DOMAIN, error_log};
use crate::{
    gateway::{
        chain::{Middleware, Next},
        context::Context,
        response::{BoxBodyType, ResponseBuilder},
    },
    hls::{HlsConfig, HlsManager},
};

lazy_static! {
    static ref HLS_PATH_REGEX: Regex =
        Regex::new(concat!(r"^/Videos/([^/]+)/(.+)$"))
            .expect("Invalid regex pattern");
}

#[derive(Clone)]
pub struct HlsMiddleware {
    pub state: Arc<AppState>,
}

impl HlsMiddleware {
    #[allow(dead_code)]
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    fn parse_path<'a>(&self, path: &'a str) -> Option<(&'a str, &'a str)> {
        let captures = HLS_PATH_REGEX.captures(path)?;
        let id = captures.get(1)?.as_str();
        let file = captures.get(2)?.as_str();
        Some((id, file))
    }

    async fn get_original_path(&self, id: &str) -> Option<PathBuf> {
        self.state.get_hls_info_cache().await.get::<PathBuf>(id)
    }

    async fn ensure_hls_stream(
        &self,
        original_path: &Path,
    ) -> Result<PathBuf, String> {
        let transcode_root_path =
            self.state.get_hls_path_cache().await.to_path_buf();
        let hls_config = HlsConfig {
            transcode_root_path,
            segment_duration_seconds: 6,
        };
        let hls_manager = HlsManager::new(self.state.clone(), hls_config);
        hls_manager.ensure_stream(original_path).await?;
        hls_manager.get_manifest_path(original_path)
    }
}

#[async_trait]
impl Middleware for HlsMiddleware {
    async fn handle(
        &self,
        ctx: Context,
        body: Option<Incoming>,
        next: Next,
    ) -> Response<BoxBodyType> {
        let (item_id, requested_file) = match self.parse_path(&ctx.path) {
            Some((id, file)) => (id, file),
            None => return next(ctx, body).await,
        };

        let original_path = match self.get_original_path(item_id).await {
            Some(path) => path,
            None => {
                error_log!(
                    STREAM_LOGGER_DOMAIN,
                    "No path mapping for ID: {}",
                    item_id
                );
                return ResponseBuilder::with_status_code(
                    StatusCode::NOT_FOUND,
                );
            }
        };

        let manifest_path = match self.ensure_hls_stream(&original_path).await {
            Ok(path) => path,
            Err(e) => {
                error_log!(
                    STREAM_LOGGER_DOMAIN,
                    "HLS stream ensuring failed for ID {}: {}",
                    item_id,
                    e
                );
                return ResponseBuilder::with_status_code(
                    StatusCode::INTERNAL_SERVER_ERROR,
                );
            }
        };

        let cache_dir = manifest_path.parent().unwrap();
        let requested_file_path =
            cache_dir.join(requested_file.replace("hls/", ""));

        const MAX_RETRIES: u32 = 10;
        const RETRY_DELAY: Duration = Duration::from_millis(500);

        if let Some(found_path) =
            wait_for_file(&requested_file_path, MAX_RETRIES, RETRY_DELAY).await
        {
            return serve_static_file(&found_path).await;
        }

        error_log!(
            STREAM_LOGGER_DOMAIN,
            "HLS segment not found after waiting: {:?}",
            requested_file_path
        );
        ResponseBuilder::with_status_code(StatusCode::NOT_FOUND)
    }

    fn clone_box(&self) -> Box<dyn Middleware> {
        Box::new(self.clone())
    }
}

async fn wait_for_file(
    path: &Path,
    retries: u32,
    delay: Duration,
) -> Option<PathBuf> {
    for _ in 0..retries {
        if path.exists() {
            return Some(path.to_path_buf());
        }
        sleep(delay).await;
    }
    None
}

async fn serve_static_file(file_path: &Path) -> Response<BoxBodyType> {
    match tokio::fs::read(file_path).await {
        Ok(contents) => {
            let mut builder = Response::builder().status(StatusCode::OK);

            let content_type =
                match file_path.extension().and_then(|s| s.to_str()) {
                    Some("m3u8") => "application/vnd.apple.mpegurl",
                    Some("ts") => "video/mp2t",
                    Some("vtt") => "text/vtt",
                    _ => "application/octet-stream",
                };

            builder = builder
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, "public, max-age=31536000");

            builder
                .body(
                    Full::new(Bytes::from(contents))
                        .map_err(|e| match e {})
                        .boxed(),
                )
                .unwrap()
        }
        Err(e) => {
            error_log!(
                STREAM_LOGGER_DOMAIN,
                "Failed to read HLS static file {:?}: {}",
                file_path,
                e
            );
            ResponseBuilder::with_status_code(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
