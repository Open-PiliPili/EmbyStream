use std::{
    collections::HashMap,
    fs::File as StdFile,
    io::{Error as IoError, ErrorKind},
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    time::{Instant, SystemTime},
};

use bytes::Bytes;
use futures_util::{StreamExt, TryStreamExt};
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use hyper::{HeaderMap, StatusCode, header};
use lazy_static::lazy_static;

use super::{
    read_stream::ReaderStream,
    response::Response,
    result::Result as AppStreamResult,
    types::{
        ClientInfo, ContentRange, PreparedLocalStreamTarget, RangeParseError,
    },
};
use crate::cache::{FileMetadata, RateLimiter};
use crate::gateway::error::Error as GatewayError;
use crate::util::string_util::StringUtil;
use crate::{
    AppState, LOCAL_STREAMER_LOGGER_DOMAIN, debug_log, error_log, info_log,
    warn_log,
};

pub(crate) struct LocalStreamer;

const LOCAL_METADATA_CACHE_KEY_PREFIX: &str = "backend:local_metadata";

#[derive(Debug, Clone, Copy)]
struct MetadataLoadStats {
    cache_hit: bool,
    lock_wait_ms: u128,
    metadata_ms: u128,
}

impl LocalStreamer {
    pub async fn stream(
        state: Arc<AppState>,
        path: PathBuf,
        mut range_header: Option<String>,
        client_info: ClientInfo,
        node_uuid: &str,
    ) -> Result<AppStreamResult, StatusCode> {
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
        let playback_session_id = match client_info.playback_session_id {
            Some(value) if !value.is_empty() => value,
            _ => {
                error_log!(
                    LOCAL_STREAMER_LOGGER_DOMAIN,
                    "Empty playback session id for '{:?}'",
                    &path,
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        let limiter = match state.get_rate_limiter_cache(node_uuid).await {
            Some(cache) => cache.fetch_limiter(&client_id_value).await,
            None => {
                info_log!(
                    LOCAL_STREAMER_LOGGER_DOMAIN,
                    "local_stream_unlimited_no_limiter_cache node_uuid={} path={:?} hint=non-Disk_or_unknown_node",
                    node_uuid,
                    path
                );
                RateLimiter::unlimited()
            }
        };

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

        info_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "local_stream_session device_id={} session_id={} path={:?} range={}",
            client_id_value,
            playback_session_id,
            path,
            range_value
        );

        let prepared_target =
            Self::prepare_stream_target(state.clone(), path).await?;

        let content_range = match Self::parse_content_range(
            range_value,
            prepared_target.file_metadata.file_size,
        ) {
            Ok(range) => {
                debug_log!(
                    LOCAL_STREAMER_LOGGER_DOMAIN,
                    "Successfully parsed content range: {:?} for path: {:?}",
                    range,
                    &prepared_target.path
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
            prepared_target,
            content_range,
            StatusCode::PARTIAL_CONTENT,
            limiter,
        )
        .await
    }

    async fn prepare_stream_target(
        state: Arc<AppState>,
        path: PathBuf,
    ) -> Result<PreparedLocalStreamTarget, StatusCode> {
        match Self::prepare_direct_target(state.clone(), &path).await {
            Ok(target) => Ok(target),
            Err(primary_err) => {
                warn_log!(
                    LOCAL_STREAMER_LOGGER_DOMAIN,
                    "primary_stream_target_unavailable path={:?} error={} \
                     hint=trying_fallback_video",
                    path,
                    primary_err
                );

                let Some(fallback_path) =
                    Self::fallback_path(state.clone()).await
                else {
                    return Err(StatusCode::NOT_FOUND);
                };

                if fallback_path == path {
                    return Err(StatusCode::NOT_FOUND);
                }

                match Self::prepare_direct_target(state.clone(), &fallback_path)
                    .await
                {
                    Ok(target) => {
                        warn_log!(
                            LOCAL_STREAMER_LOGGER_DOMAIN,
                            "Using fallback video for unavailable path={:?} \
                             fallback={:?}",
                            path,
                            fallback_path
                        );
                        Ok(target.with_fallback(true))
                    }
                    Err(fallback_err) => {
                        error_log!(
                            LOCAL_STREAMER_LOGGER_DOMAIN,
                            "fallback_stream_target_unavailable path={:?} \
                             fallback={:?} error={}",
                            path,
                            fallback_path,
                            fallback_err
                        );
                        Err(StatusCode::NOT_FOUND)
                    }
                }
            }
        }
    }

    async fn prepare_direct_target(
        state: Arc<AppState>,
        path: &Path,
    ) -> Result<PreparedLocalStreamTarget, IoError> {
        const SLOW_PREPARE_TARGET_THRESHOLD_MS: u128 = 500;

        let prepare_started = Instant::now();
        let probe_path = path.to_path_buf();
        let open_started = Instant::now();
        let opened_file =
            tokio::task::spawn_blocking(move || StdFile::open(&probe_path))
                .await
                .map_err(|err| {
                    IoError::other(format!(
                        "blocking open task failed for {:?}: {}",
                        path, err
                    ))
                })??;
        let open_ms = open_started.elapsed().as_millis();
        let metadata_started = Instant::now();
        let (file_metadata, metadata_stats) =
            Self::load_cached_file_metadata(state, path, &opened_file).await?;
        let metadata_total_ms = metadata_started.elapsed().as_millis();
        let prepare_ms = prepare_started.elapsed().as_millis();

        if prepare_ms >= SLOW_PREPARE_TARGET_THRESHOLD_MS {
            info_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "local_prepare_target_slow elapsed_ms={} open_ms={} metadata_total_ms={} metadata_io_ms={} lock_wait_ms={} cache_hit={} path={:?}",
                prepare_ms,
                open_ms,
                metadata_total_ms,
                metadata_stats.metadata_ms,
                metadata_stats.lock_wait_ms,
                metadata_stats.cache_hit,
                path
            );
        } else {
            debug_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "local_prepare_target_complete elapsed_ms={} open_ms={} metadata_total_ms={} metadata_io_ms={} lock_wait_ms={} cache_hit={} path={:?}",
                prepare_ms,
                open_ms,
                metadata_total_ms,
                metadata_stats.metadata_ms,
                metadata_stats.lock_wait_ms,
                metadata_stats.cache_hit,
                path
            );
        }

        Ok(
            PreparedLocalStreamTarget::new(path.to_path_buf(), file_metadata)
                .with_opened_file(opened_file),
        )
    }

    async fn load_cached_file_metadata(
        state: Arc<AppState>,
        path: &Path,
        opened_file: &StdFile,
    ) -> Result<(FileMetadata, MetadataLoadStats), IoError> {
        let cache_key = Self::local_metadata_cache_key(path);
        let cache = state.get_local_metadata_cache().await;

        if let Some(cached) = cache.get::<FileMetadata>(&cache_key) {
            debug_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "local_metadata_cache_hit key={} path={:?}",
                cache_key,
                path
            );
            return Ok((
                cached,
                MetadataLoadStats {
                    cache_hit: true,
                    lock_wait_ms: 0,
                    metadata_ms: 0,
                },
            ));
        }

        let cache_lock = AppState::request_lock(
            &state.local_metadata_request_locks,
            &cache_key,
        );
        let lock_started = Instant::now();
        let guard = cache_lock.lock().await;
        let lock_wait_ms = lock_started.elapsed().as_millis();

        if let Some(cached) = cache.get::<FileMetadata>(&cache_key) {
            debug_log!(
                LOCAL_STREAMER_LOGGER_DOMAIN,
                "local_metadata_inflight_wait_hit key={} path={:?}",
                cache_key,
                path
            );
            drop(guard);
            AppState::cleanup_request_lock(
                &state.local_metadata_request_locks,
                &cache_key,
                &cache_lock,
            );
            return Ok((
                cached,
                MetadataLoadStats {
                    cache_hit: true,
                    lock_wait_ms,
                    metadata_ms: 0,
                },
            ));
        }

        let metadata_started = Instant::now();
        let metadata = opened_file.metadata()?;
        let metadata_ms = metadata_started.elapsed().as_millis();
        if !metadata.is_file() {
            drop(guard);
            AppState::cleanup_request_lock(
                &state.local_metadata_request_locks,
                &cache_key,
                &cache_lock,
            );
            return Err(IoError::new(
                ErrorKind::NotFound,
                format!("path is not a file: {:?}", path),
            ));
        }

        let file_metadata = FileMetadata {
            file_size: metadata.len(),
            file_name: path
                .file_name()
                .and_then(|s| s.to_str())
                .map_or_else(|| "unknown".to_string(), |s| s.to_string()),
            format: path
                .extension()
                .and_then(|s| s.to_str())
                .map_or_else(|| "unknown".to_string(), |s| s.to_string()),
            last_modified: metadata.modified().ok(),
            updated_at: SystemTime::now(),
        };

        cache.insert(cache_key.clone(), file_metadata.clone());
        debug_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "local_metadata_cache_store key={} path={:?}",
            cache_key,
            path
        );

        drop(guard);
        AppState::cleanup_request_lock(
            &state.local_metadata_request_locks,
            &cache_key,
            &cache_lock,
        );

        Ok((
            file_metadata,
            MetadataLoadStats {
                cache_hit: false,
                lock_wait_ms,
                metadata_ms,
            },
        ))
    }

    fn local_metadata_cache_key(path: &Path) -> String {
        let raw_path = path.to_string_lossy();
        let path_hash = StringUtil::hash_hex(raw_path.trim_end());
        format!("{LOCAL_METADATA_CACHE_KEY_PREFIX}:path_hash:{path_hash}")
    }

    async fn fallback_path(state: Arc<AppState>) -> Option<PathBuf> {
        let config = state.get_config().await;
        let fallback_path_str = &config.fallback.video_missing_path;
        if fallback_path_str.is_empty() {
            return None;
        }

        let fallback_path = PathBuf::from(fallback_path_str);
        if fallback_path.is_absolute() {
            Some(fallback_path)
        } else {
            Some(
                config
                    .path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(fallback_path),
            )
        }
    }

    async fn stream_file(
        prepared_target: PreparedLocalStreamTarget,
        content_range: ContentRange,
        status_code: StatusCode,
        limiter: Arc<RateLimiter>,
    ) -> Result<AppStreamResult, StatusCode> {
        let PreparedLocalStreamTarget {
            path,
            file_metadata,
            opened_file,
            ..
        } = prepared_target;

        info_log!(
            LOCAL_STREAMER_LOGGER_DOMAIN,
            "Streaming file status {:?}, range: {:?}",
            status_code,
            content_range,
        );

        type Framed = Pin<
            Box<
                dyn futures_util::Stream<
                        Item = Result<Frame<Bytes>, GatewayError>,
                    > + Send
                    + Sync,
            >,
        >;

        let reader_stream = match opened_file {
            Some(opened_file) => ReaderStream::from_opened_file(
                path.clone(),
                opened_file,
                content_range,
            ),
            None => ReaderStream::new(path.clone(), content_range),
        };

        let stream: Framed = if limiter.skip_semaphore {
            let s = reader_stream
                .into_stream()
                .map(|res| res.map(Frame::data).map_err(GatewayError::from));
            Box::pin(s)
        } else {
            let sem = limiter.semaphore.clone();
            let s = reader_stream
                .into_stream()
                .and_then(move |chunk| {
                    let sem = sem.clone();
                    async move {
                        match sem.acquire_many(chunk.len() as u32).await {
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
                .map_err(GatewayError::from);
            Box::pin(s)
        };

        let mut headers = HeaderMap::new();
        if let Ok(content_type) =
            get_content_type(&file_metadata.format).parse()
        {
            headers.insert(header::CONTENT_TYPE, content_type);
        }
        if let Ok(accept_ranges) = "bytes".parse() {
            headers.insert(header::ACCEPT_RANGES, accept_ranges);
        }

        if status_code == StatusCode::PARTIAL_CONTENT {
            headers
                .insert(header::CONTENT_LENGTH, content_range.length().into());
            let range_str = format!(
                "bytes {}-{}/{}",
                content_range.start,
                content_range.end,
                content_range.total_size
            );
            if let Ok(range_value) = range_str.parse() {
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

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use super::LocalStreamer;
    use crate::{
        AppState,
        config::core::{finish_raw_config, parse_raw_config_str},
        core::backend::{result::Result as AppStreamResult, types::ClientInfo},
        util::string_util::StringUtil,
    };

    async fn test_state_with_fallback(
        fallback_path: Option<&std::path::Path>,
    ) -> AppState {
        let fallback_value = fallback_path
            .map(|path| path.to_string_lossy().replace('\\', "\\\\"))
            .unwrap_or_default();
        let raw = format!(
            r#"
[Log]
level = "info"
prefix = ""
root_path = "./logs"

[General]
memory_mode = "middle"
stream_mode = "frontend"
encipher_key = "1234567890123456"
encipher_iv = "1234567890123456"

[Emby]
url = "http://127.0.0.1"
port = "8096"
token = "tok"

[UserAgent]
mode = "allow"
allow_ua = []
deny_ua = []

[Fallback]
video_missing_path = "{fallback_value}"

[Frontend]
listen_port = 60001
"#
        );
        let parsed = match parse_raw_config_str(&raw) {
            Ok(parsed) => parsed,
            Err(err) => panic!("parse raw config failed: {err}"),
        };
        let config = match finish_raw_config(PathBuf::from("test.toml"), parsed)
        {
            Ok(config) => config,
            Err(err) => panic!("finish raw config failed: {err}"),
        };
        AppState::new(config).await
    }

    fn write_test_file(dir: &TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        assert!(fs::write(&path, b"hello world").is_ok());
        path
    }

    #[tokio::test]
    async fn prepare_stream_target_prefers_primary_path() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = write_test_file(&dir, "primary.mp4");
        let fallback_path = write_test_file(&dir, "fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let target =
            LocalStreamer::prepare_stream_target(state, primary_path.clone())
                .await
                .unwrap_or_else(|err| panic!("prepare primary target: {err}"));

        assert_eq!(target.path, primary_path);
        assert!(!target.is_fallback);
        assert!(target.has_opened_file());
    }

    #[test]
    fn local_metadata_cache_key_is_structured() {
        let path = PathBuf::from("/tmp/media/Foo Bar.mp4");
        let key = LocalStreamer::local_metadata_cache_key(&path);
        let expected_hash =
            StringUtil::hash_hex(path.to_string_lossy().trim_end());

        assert_eq!(
            key,
            format!("backend:local_metadata:path_hash:{expected_hash}")
        );
    }

    #[tokio::test]
    async fn prepare_stream_target_reuses_cached_metadata() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = write_test_file(&dir, "primary.mp4");
        let state = std::sync::Arc::new(test_state_with_fallback(None).await);

        let first = LocalStreamer::prepare_stream_target(
            state.clone(),
            primary_path.clone(),
        )
        .await
        .unwrap_or_else(|err| panic!("first prepare target: {err}"));
        let key = LocalStreamer::local_metadata_cache_key(&primary_path);
        let cache = state.get_local_metadata_cache().await;
        let cached = cache
            .get::<crate::cache::FileMetadata>(&key)
            .unwrap_or_else(|| panic!("metadata cached after first prepare"));

        let second = LocalStreamer::prepare_stream_target(state, primary_path)
            .await
            .unwrap_or_else(|err| panic!("second prepare target: {err}"));

        assert_eq!(cached.file_size, first.file_metadata.file_size);
        assert_eq!(cached.updated_at, first.file_metadata.updated_at);
        assert_eq!(second.file_metadata.updated_at, cached.updated_at);
    }

    #[tokio::test]
    async fn prepare_stream_target_uses_fallback_when_primary_missing() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = dir.path().join("missing.mp4");
        let fallback_path = write_test_file(&dir, "fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let target = LocalStreamer::prepare_stream_target(state, primary_path)
            .await
            .unwrap_or_else(|err| panic!("prepare fallback target: {err}"));

        assert_eq!(target.path, fallback_path);
        assert!(target.is_fallback);
        assert!(target.has_opened_file());
    }

    #[tokio::test]
    async fn prepare_stream_target_returns_not_found_when_primary_and_fallback_fail()
     {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = dir.path().join("missing.mp4");
        let fallback_path = dir.path().join("missing-fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let err = LocalStreamer::prepare_stream_target(state, primary_path)
            .await
            .expect_err("missing primary and fallback should fail");

        assert_eq!(err, hyper::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn stream_returns_partial_content_for_primary_target() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = write_test_file(&dir, "primary.mp4");
        let fallback_path = write_test_file(&dir, "fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let result = LocalStreamer::stream(
            state,
            primary_path,
            Some("bytes=0-".to_string()),
            ClientInfo::new(
                Some("client-1".to_string()),
                Some("play-123-1".to_string()),
                None,
                None,
            ),
            "test-node",
        )
        .await
        .unwrap_or_else(|err| panic!("stream primary file: {err}"));

        match result {
            AppStreamResult::Stream(response) => {
                assert_eq!(response.status, hyper::StatusCode::PARTIAL_CONTENT);
            }
            AppStreamResult::Redirect(_) => {
                unreachable!("expected stream response")
            }
        }
    }

    #[tokio::test]
    async fn stream_uses_fallback_when_primary_missing() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = dir.path().join("missing.mp4");
        let fallback_path = write_test_file(&dir, "fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let result = LocalStreamer::stream(
            state,
            primary_path,
            Some("bytes=0-".to_string()),
            ClientInfo::new(
                Some("client-1".to_string()),
                Some("play-123-1".to_string()),
                None,
                None,
            ),
            "test-node",
        )
        .await
        .unwrap_or_else(|err| panic!("stream fallback file: {err}"));

        match result {
            AppStreamResult::Stream(response) => {
                assert_eq!(response.status, hyper::StatusCode::PARTIAL_CONTENT);
            }
            AppStreamResult::Redirect(_) => {
                unreachable!("expected stream response")
            }
        }
    }

    #[tokio::test]
    async fn stream_returns_not_found_when_primary_and_fallback_missing() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = dir.path().join("missing.mp4");
        let fallback_path = dir.path().join("missing-fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let err = LocalStreamer::stream(
            state,
            primary_path,
            Some("bytes=0-".to_string()),
            ClientInfo::new(
                Some("client-1".to_string()),
                Some("play-123-1".to_string()),
                None,
                None,
            ),
            "test-node",
        )
        .await;

        match err {
            Ok(_) => unreachable!("missing primary and fallback should fail"),
            Err(status) => assert_eq!(status, hyper::StatusCode::NOT_FOUND),
        }
    }

    #[tokio::test]
    async fn stream_rejects_malformed_range() {
        let dir = match TempDir::new() {
            Ok(dir) => dir,
            Err(err) => panic!("temp dir failed: {err}"),
        };
        let primary_path = write_test_file(&dir, "primary.mp4");
        let fallback_path = write_test_file(&dir, "fallback.mp4");
        let state = std::sync::Arc::new(
            test_state_with_fallback(Some(&fallback_path)).await,
        );

        let err = LocalStreamer::stream(
            state,
            primary_path,
            Some("bytes=bad".to_string()),
            ClientInfo::new(
                Some("client-1".to_string()),
                Some("play-123-1".to_string()),
                None,
                None,
            ),
            "test-node",
        )
        .await;

        match err {
            Ok(_) => unreachable!("malformed range should fail"),
            Err(status) => assert_eq!(status, hyper::StatusCode::BAD_REQUEST),
        }
    }
}
