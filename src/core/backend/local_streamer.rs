use std::{
    collections::HashMap,
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
    sync::Arc,
};

use futures_util::TryStreamExt;
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::{
    HeaderMap, StatusCode,
    header::{self, HeaderValue},
};
use lazy_static::lazy_static;
use tokio::sync::Semaphore;

use super::{
    read_stream::ReaderStream,
    response::Response,
    result::Result as AppStreamResult,
    types::{ClientInfo, ContentRange, RangeParseError},
};
use crate::cache::FileMetadata;
use crate::{
    AppState, LOCAL_STREAMER_LOGGER_DOMAIN, debug_log, error_log, info_log,
    warn_log,
};

pub(crate) struct LocalStreamer;

impl LocalStreamer {
    pub async fn stream(
        state: Arc<AppState>,
        path: PathBuf,
        mut range_header: Option<String>,
        client_info: ClientInfo,
    ) -> Result<AppStreamResult, StatusCode> {
        if !path.is_file() {
            return Err(StatusCode::NOT_FOUND);
        }

        let client_id_value = match client_info.id {
            Some(value) if !value.is_empty() => value,
            _ => {
                error_log!(
                    LOCAL_STREAMER_LOGGER_DOMAIN,
                    "Empty client id for '{:?}'",
                    &path,
                );
                return Err(StatusCode::FORBIDDEN);
            }
        };

        let rate_limiter_cache = state.get_rate_limiter_cache().await;
        let limiter = rate_limiter_cache.fetch_limiter(&client_id_value).await;

        let problematic_clients = state.get_problematic_clients().await;

        Self::fix_range_header_if_needed(
            &mut range_header,
            &client_info.user_agent,
            problematic_clients,
        )
        .await;

        let Some(range_value) = range_header.as_deref() else {
            error_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "No-Range req for '{:?}' rejected. IP: {:?}, Client: {:?}, ClientID: {:?}",
                &path,
                client_info.ip,
                client_info.user_agent,
                client_id_value,
            );
            return Err(StatusCode::FORBIDDEN);
        };

        let cache = state.get_metadata_cache().await;
        let Ok(file_metadata) = cache.fetch_metadata(&path).await else {
            error_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "Failed to obtain cache metadata for the route: {:?}",
                &path
            );
            return Err(StatusCode::NOT_FOUND);
        };

        let content_range = match Self::parse_content_range(
            range_value,
            file_metadata.file_size,
        ) {
            Ok(range) => {
                debug_log!(
                    LOCAL_STREAMER_LOGGER_DOMAIN,
                    "Successfully parsed content range: {:?} for path: {:?}",
                    range,
                    &path
                );
                range
            }
            Err(RangeParseError::Malformed) => {
                return Err(StatusCode::BAD_REQUEST);
            }
            Err(RangeParseError::Unsatisfiable) => {
                return Err(StatusCode::RANGE_NOT_SATISFIABLE);
            }
        };

        Self::stream_file(
            &path,
            &file_metadata,
            content_range,
            StatusCode::PARTIAL_CONTENT,
            limiter.semaphore.clone(),
        )
        .await
    }

    async fn stream_file(
        path: &Path,
        file_metadata: &FileMetadata,
        content_range: ContentRange,
        status_code: StatusCode,
        semaphore: Arc<Semaphore>,
    ) -> Result<AppStreamResult, StatusCode> {
        info_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Streaming file status {:?}, range: {:?}",
            status_code,
            content_range,
        );

        let stream = ReaderStream::new(path, content_range)
            .into_stream()
            .and_then(move |chunk| {
                let semaphore_clone = semaphore.clone();
                async move {
                    match semaphore_clone.acquire_many(chunk.len() as u32).await
                    {
                        Ok(permit) => {
                            permit.forget();
                            Ok(chunk)
                        }
                        Err(_) => Err(IoError::new(
                            ErrorKind::BrokenPipe,
                            "Semaphore closed",
                        )),
                    }
                }
            })
            .map_ok(Frame::data)
            .map_err(Into::into);

        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(get_content_type(&file_metadata.format)),
        );
        headers
            .insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));

        if status_code == StatusCode::PARTIAL_CONTENT {
            headers
                .insert(header::CONTENT_LENGTH, content_range.length().into());
            let range_str = format!(
                "bytes {}-{}/{}",
                content_range.start,
                content_range.end,
                content_range.total_size
            );
            if let Ok(range_value) = HeaderValue::from_str(&range_str) {
                headers.insert(header::CONTENT_RANGE, range_value);
            }
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

    /// Handles requests from specific clients that do not send a Range header by applying a default.
    ///
    /// # WARNING: Temporary Workaround
    /// This method serves as a temporary compatibility layer for clients (e.g., `yamby`, `hills`)
    /// that incorrectly omit the `Range` header in streaming requests.
    ///
    /// This is considered a tactical fix. The correct long-term solution is for the client applications
    /// to solve this issue. This workaround may be deprecated or removed in future releases
    /// as clients become compliant.
    async fn fix_range_header_if_needed(
        range_header: &mut Option<String>,
        client: &Option<String>,
        problematic_clients: &[String],
    ) {
        if let Some(header) = range_header {
            if !header.is_empty() {
                return;
            }
        }

        let Some(client_str) = client else {
            return;
        };

        let client_lower = client_str.to_lowercase();

        // Check if the client user agent contains any of the known problematic substrings.
        if problematic_clients.iter().any(|c| client_lower.contains(c)) {
            warn_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "Client '{:?}' missing Range header. Applying workaround 'bytes=0-'.",
                client_str
            );
            *range_header = Some("bytes=0-".to_string());
        }
    }

    fn parse_content_range(
        range_value: &str,
        total_size: u64,
    ) -> Result<ContentRange, RangeParseError> {
        debug_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Start parsing content range: {:?}",
            range_value
        );

        let ranges = http_range_header::parse_range_header(range_value)
            .map_err(|_| RangeParseError::Malformed)?;

        let validated_ranges = ranges
            .validate(total_size)
            .map_err(|_| RangeParseError::Unsatisfiable)?;

        if let Some(first_range) = validated_ranges.first() {
            Ok(ContentRange {
                start: *first_range.start(),
                end: *first_range.end(),
                total_size,
            })
        } else {
            Err(RangeParseError::Unsatisfiable)
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
