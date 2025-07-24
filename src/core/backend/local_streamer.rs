use std::{collections::HashMap, io, path::PathBuf, sync::Arc};

use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::{HeaderMap, StatusCode, header};
use lazy_static::lazy_static;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio_util::io::ReaderStream;

use super::{
    response::Response, result::Result as AppStreamResult, types::ContentRange,
};
use crate::cache::FileMetadata;
use crate::{
    AppState, LOCAL_STREAMER_LOGGER_DOMAIN, cache::FileEntry, error_log,
    info_log,
};

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

        let Ok(file_metadata) = cache.fetch_metadata(&path).await else {
            error_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "Failed to obtain cache metadata for the route: {:?}",
                &path
            );
            return Err(StatusCode::NOT_FOUND);
        };

        let content_range = Self::parse_content_range(
            range_header.as_deref(),
            file_metadata.file_size,
        );
        let status_code =
            if range_header.is_some() && !content_range.is_full_range() {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };

        Self::stream_file(
            &file_entry,
            &file_metadata,
            content_range,
            status_code,
        )
        .await
    }

    async fn stream_file(
        file_entry: &FileEntry,
        file_metadata: &FileMetadata,
        content_range: ContentRange,
        status_code: StatusCode,
    ) -> Result<AppStreamResult, StatusCode> {
        let mut file = file_entry
            .handle
            .read()
            .await
            .try_clone()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        file.seek(io::SeekFrom::Start(content_range.start))
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        info_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Streaming file with status {:?}, seek content_range: {:?}",
            status_code,
            content_range,
        );

        let limited_reader = file.take(content_range.length());
        const MB: usize = 1024 * 1024;
        let buffer: usize = if status_code == StatusCode::PARTIAL_CONTENT {
            4 * MB
        } else {
            1 * MB
        };
        let stream = ReaderStream::with_capacity(limited_reader, buffer)
            .map_ok(Frame::data)
            .map_err(Into::into);

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            get_content_type(&file_metadata.format).parse().unwrap(),
        );
        headers.insert(header::ACCEPT_RANGES, "bytes".parse().unwrap());

        if status_code == StatusCode::PARTIAL_CONTENT {
            headers
                .insert(header::CONTENT_LENGTH, content_range.length().into());
            let range_str = format!(
                "bytes {}-{}/{}",
                content_range.start,
                content_range.end,
                content_range.total_size
            );
            headers.insert(header::CONTENT_RANGE, range_str.parse().unwrap());
        } else {
            headers.insert(
                header::CONTENT_LENGTH,
                content_range.total_size.into(),
            );
        }

        let response = Response {
            status: status_code,
            headers,
            body: BodyExt::boxed(StreamBody::new(stream)),
        };

        Ok(AppStreamResult::Stream(response))
    }

    fn parse_content_range(
        range_header: Option<&str>,
        total_size: u64,
    ) -> ContentRange {
        let (start, end) = range_header
            .and_then(|header_value| {
                http_range_header::parse_range_header(header_value).ok()
            })
            .and_then(|parsed| parsed.validate(total_size).ok())
            .and_then(|validated| {
                validated.first().map(|r| (*r.start(), *r.end()))
            })
            .unwrap_or((0, total_size.saturating_sub(1)));

        ContentRange {
            start,
            end,
            total_size,
        }
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
