use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::{HeaderMap, StatusCode, header};
use lazy_static::lazy_static;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use super::{response::Response, result::Result as AppStreamResult};
use crate::cache::FileMetadata;
use crate::{
    AppState, LOCAL_STREAMER_LOGGER_DOMAIN, cache::FileEntry, debug_log,
    error_log,
};

const CHUNK_SIZE: usize = 128 * 1024;

pub(crate) struct LocalStreamer;

impl LocalStreamer {
    pub async fn stream(
        state: Arc<AppState>,
        path: PathBuf,
        range_header: Option<String>,
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
            Self::stream_partial_content(&file_entry, &metadata, &range_value)
                .await
        } else {
            Self::stream_full_file(&file_entry, &metadata).await
        }
    }

    async fn stream_partial_content(
        file_entry: &FileEntry,
        file_metadata: &FileMetadata,
        range_value: &str,
    ) -> Result<AppStreamResult, StatusCode> {
        debug_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Start stream partial content, metadata: {:?}, range_value: {:?}",
            file_metadata,
            range_value
        );

        let Ok(parsed) = http_range_header::parse_range_header(range_value)
        else {
            return Err(StatusCode::RANGE_NOT_SATISFIABLE);
        };

        let file_length = file_metadata.file_size;
        let Ok(validated_ranges) = parsed.validate(file_length) else {
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

        debug_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Successfully seeked stream partial content, start: {}, end: {}, len: {}",
            start,
            end,
            len
        );

        let limited_reader = file.take(len);
        let stream = ReaderStream::with_capacity(limited_reader, CHUNK_SIZE)
            .map_ok(Frame::data)
            .map_err(Into::into);

        let mut headers = HeaderMap::new();
        let content_type = get_content_type(file_metadata.format.as_str());
        let content_range =
            format!("bytes {}-{}/{}", start, end, file_metadata.file_size);

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
    ) -> Result<AppStreamResult, StatusCode> {
        debug_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Start stream full content, metadata: {:?}",
            file_metadata
        );

        let handle = file_entry.handle.read().await;
        let file = handle
            .try_clone()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let stream = ReaderStream::with_capacity(file, CHUNK_SIZE)
            .map_ok(Frame::data)
            .map_err(Into::into);

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
