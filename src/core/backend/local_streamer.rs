use std::{collections::HashMap, io, path::PathBuf, sync::Arc, time::Instant};

use futures_util::{StreamExt, TryStreamExt};
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::{HeaderMap, StatusCode, header};
use lazy_static::lazy_static;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

use super::{
    chunk_stream::AdaptiveChunkStream, response::Response, result::Result as AppStreamResult,
};
use crate::cache::FileMetadata;
use crate::{AppState, cache::FileEntry, error_log, LOCAL_STREAMER_LOGGER_DOMAIN};

pub(crate) struct LocalStreamer;

impl LocalStreamer {
    pub async fn stream(
        state: Arc<AppState>,
        path: PathBuf,
        range_header: Option<String>,
        start_time: Instant,
    ) -> Result<AppStreamResult, StatusCode> {
        if !path.is_file() {
            return Err(StatusCode::NOT_FOUND);
        }

        let cache = state.get_file_cache().await;

        let Ok(file_entry) = cache.fetch_entry(&path).await else {
            error_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "Failed to obtain cache entry for the route: {:?}",
                &path
            );
            return Err(StatusCode::NOT_FOUND);
        };

        let Ok(metadata) = cache.fetch_metadata(&path).await else {
            error_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "Failed to obtain cache metadata for the route: {:?}",
                &path
            );
            return Err(StatusCode::NOT_FOUND);
        };

        if let Some(range_value) = range_header {
            Self::stream_partial_content(&file_entry, &metadata, &range_value, start_time).await
        } else {
            Self::stream_full_file(&file_entry, &metadata, start_time).await
        }
    }

    async fn stream_partial_content(
        file_entry: &FileEntry,
        file_metadata: &FileMetadata,
        range_value: &str,
        start_time: Instant,
    ) -> Result<AppStreamResult, StatusCode> {
        let Ok(parsed) = http_range_header::parse_range_header(range_value) else {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE);
        };

        let Ok(validated_ranges) = parsed.validate(file_metadata.file_size) else {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE);
        };

        let Some(range) = validated_ranges.first() else {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE);
        };

        let start = range.start();
        let end = range.end();
        let len = end - start + 1;

        let handle = file_entry.handle.read().await;
        let mut file = handle
            .try_clone()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        file.seek(io::SeekFrom::Start(*start))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let limited_reader = file.take(len);
        let stream = AdaptiveChunkStream::new(limited_reader, start_time)
            .map(|res| res.map(Frame::data))
            .map_err(|e| e.into());

        let mut headers = HeaderMap::new();
        let content_type = get_content_type(file_metadata.format.as_str());
        let content_range = format!("bytes {}-{}/{}", start, end, file_metadata.file_size);

        headers.insert(header::CONTENT_LENGTH, len.into());
        if let Ok(accept_ranges) = content_range.parse() {
            headers.insert(header::ACCEPT_RANGES, accept_ranges);
        }
        if let Ok(content_type) = content_type.parse() {
            headers.insert(header::CONTENT_TYPE, content_type);
        }
        if let Ok(content_range) = content_range.parse() {
            headers.insert(header::CONTENT_RANGE, content_range);
        }

        let response = Response {
            status: StatusCode::PARTIAL_CONTENT,
            headers,
            body: BodyExt::boxed(StreamBody::new(stream)),
        };

        Ok(AppStreamResult::Stream(response))
    }

    async fn stream_full_file(
        file_entry: &FileEntry,
        file_metadata: &FileMetadata,
        start_time: Instant,
    ) -> Result<AppStreamResult, StatusCode> {
        let handle = file_entry.handle.read().await;
        let file = handle
            .try_clone()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let stream = AdaptiveChunkStream::new(file, start_time)
            .map(|res| res.map(Frame::data))
            .map_err(|e| e.into());

        let mut headers = HeaderMap::new();
        let content_type = get_content_type(file_metadata.format.as_str());

        headers.insert(header::CONTENT_LENGTH, file_metadata.file_size.into());
        if let Ok(accept_ranges) = "bytes".parse() {
            headers.insert(header::ACCEPT_RANGES, accept_ranges);
        }
        if let Ok(content_type) = content_type.parse() {
            headers.insert(header::CONTENT_TYPE, content_type);
        }

        let response = Response {
            status: StatusCode::OK,
            headers,
            body: BodyExt::boxed(StreamBody::new(stream)),
        };

        Ok(AppStreamResult::Stream(response))
    }
}

pub fn get_content_type(extension: &str) -> &'static str {
    lazy_static! {
        static ref CONTENT_TYPES: HashMap<&'static str, &'static str> = {
            let mut m = HashMap::new();
            // video
            m.insert("mp4", "video/mp4");
            m.insert("mkv", "video/x-matroska");
            m.insert("avi", "video/x-msvideo");
            m.insert("mov", "video/quicktime");
            m.insert("flv", "video/x-flv");
            m.insert("rmvb", "application/vnd.rn-realmedia-vbr");
            m.insert("rm", "application/vnd.rn-realmedia");

            // audio
            m.insert("mka", "audio/x-matroska");
            m.insert("aac", "audio/aac");
            m.insert("mp3", "audio/mpeg");
            m.insert("wav", "audio/wav");
            m.insert("ogg", "audio/ogg");

            // subtitle
            m.insert("srt", "application/x-subrip");
            m.insert("vtt", "text/vtt");
            m.insert("ass", "text/x-ssa");

            // picture
            m.insert("jpg", "image/jpeg");
            m.insert("jpeg", "image/jpeg");
            m.insert("png", "image/png");
            m.insert("gif", "image/gif");

            m
        };
    }

    let ext = extension.trim_start_matches('.').to_lowercase();
    CONTENT_TYPES
        .get(ext.as_str())
        .unwrap_or(&"application/octet-stream")
}
