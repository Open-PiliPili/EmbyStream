use std::{
    cmp::Reverse,
    collections::HashSet,
    ffi::OsStr,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Query, Request, State},
    http::{HeaderMap, StatusCode, header},
    response::Response,
    routing::get,
};
use axum_extra::extract::CookieJar;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use hyper_util::rt::TokioIo;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, handshake::derive_accept_key, protocol::Role},
};

use crate::{log_stream::LogStreamFilter, util::privacy::Privacy};

use super::{
    api::WebAppState,
    auth::session_user_from_jar,
    contracts::{LogEntry, LogListResponse, UserRole},
    db::LogsQuery,
    error::WebError,
};

const MAX_LOG_TAIL_BYTES: u64 = 2 * 1024 * 1024;

pub fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/", get(list_logs))
        .route("/stream", get(stream_logs))
}

pub async fn list_logs(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Query(query): Query<LogsQueryParams>,
) -> Result<Json<LogListResponse>, WebError> {
    let _ = query.cursor.as_deref();
    let user = session_user_from_jar(&state, &jar).await?;
    if user.role != UserRole::Admin {
        return Err(WebError::Forbidden("Administrator access is required."));
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let filter = build_log_stream_filter(&query)?;
    let mut items = state.live_logs.snapshot(&filter, limit);

    if items.len() < limit {
        let mut historical =
            load_historical_logs(&state, query.source.as_deref(), limit)
                .await?;
        items.append(&mut historical);
        normalize_log_entries(&mut items, &filter, limit);
    }

    Ok(Json(LogListResponse {
        items,
        next_cursor: None,
    }))
}

async fn stream_logs(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Query(query): Query<LogsQueryParams>,
    request: Request,
) -> Result<Response, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    if user.role != UserRole::Admin {
        return Err(WebError::Forbidden("Administrator access is required."));
    }

    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let filter = build_log_stream_filter(&query)?;
    let replay = state.live_logs.snapshot(&filter, limit);
    let receiver = state.live_logs.subscribe();
    let accept_key = websocket_accept_key(request.headers())?;

    tokio::spawn(async move {
        if let Ok(upgraded) = hyper::upgrade::on(request).await {
            let socket = WebSocketStream::from_raw_socket(
                TokioIo::new(upgraded),
                Role::Server,
                None,
            )
            .await;
            handle_logs_socket(socket, receiver, filter, replay).await;
        }
    });

    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header(header::CONNECTION, "upgrade")
        .header(header::UPGRADE, "websocket")
        .header("sec-websocket-accept", accept_key)
        .body(Body::empty())
        .map_err(|error| WebError::internal(error.to_string()))
}

fn read_named_logs(
    directory: &Path,
    limit: usize,
    source: &str,
) -> Result<Vec<LogEntry>, WebError> {
    if !directory.exists() {
        return Ok(Vec::new());
    }

    let mut paths = std::fs::read_dir(directory)
        .map_err(WebError::from)?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .filter(|path| path.is_file())
        .filter(|path| {
            let Some(file_name) = path.file_name().and_then(OsStr::to_str)
            else {
                return false;
            };

            if file_name.starts_with('.') {
                return false;
            }

            is_supported_log_file_name(file_name)
        })
        .collect::<Vec<_>>();

    paths.sort_by_key(|path| {
        Reverse(
            path.metadata()
                .and_then(|metadata| metadata.modified())
                .ok(),
        )
    });

    let mut entries = Vec::new();
    for path in paths {
        let remaining = limit.saturating_sub(entries.len());
        if remaining == 0 {
            break;
        }

        let mut file_entries = read_runtime_log_file(&path, remaining, source)?;
        entries.append(&mut file_entries);
    }

    entries.sort_by_key(|entry| Reverse(entry.timestamp));
    entries.truncate(limit);
    Ok(entries)
}

async fn load_historical_logs(
    state: &WebAppState,
    source: Option<&str>,
    limit: usize,
) -> Result<Vec<LogEntry>, WebError> {
    match source {
        Some("stream") => {
            read_named_logs(&state.config.stream_log_dir, limit, "stream")
        }
        Some("runtime") => {
            read_named_logs(&state.config.runtime_log_dir, limit, "runtime")
        }
        Some("audit") => Ok(state
            .db
            .list_logs(LogsQuery {
                source: Some("audit".to_string()),
                limit,
            })
            .await?
            .items),
        Some(_) => Err(WebError::invalid_input(
            "source",
            "Log source must be stream, runtime or audit.",
        )),
        None => {
            let mut stream =
                read_named_logs(&state.config.stream_log_dir, limit, "stream")?;
            let mut runtime = read_named_logs(
                &state.config.runtime_log_dir,
                limit,
                "runtime",
            )?;
            let mut audit = state
                .db
                .list_logs(LogsQuery {
                    source: Some("audit".to_string()),
                    limit,
                })
                .await?
                .items;
            stream.append(&mut runtime);
            stream.append(&mut audit);
            Ok(stream)
        }
    }
}

fn is_supported_log_file_name(file_name: &str) -> bool {
    file_name.ends_with(".log")
        || file_name.split('.').count() >= 2
        || is_date_only_log_file_name(file_name)
}

fn is_date_only_log_file_name(file_name: &str) -> bool {
    let mut parts = file_name.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };

    year.len() == 4
        && month.len() == 2
        && day.len() == 2
        && year.chars().all(|ch| ch.is_ascii_digit())
        && month.chars().all(|ch| ch.is_ascii_digit())
        && day.chars().all(|ch| ch.is_ascii_digit())
}

fn read_runtime_log_file(
    path: &Path,
    limit: usize,
    source: &str,
) -> Result<Vec<LogEntry>, WebError> {
    let mut file = File::open(path).map_err(WebError::from)?;
    let file_len = file.metadata().map_err(WebError::from)?.len();
    let tail_start = file_len.saturating_sub(MAX_LOG_TAIL_BYTES);

    file.seek(SeekFrom::Start(tail_start))
        .map_err(WebError::from)?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer).map_err(WebError::from)?;

    let lines = if tail_start > 0 {
        buffer
            .lines()
            .skip(1)
            .map(str::to_string)
            .collect::<Vec<_>>()
    } else {
        buffer.lines().map(str::to_string).collect::<Vec<_>>()
    };

    let mut items = lines
        .into_iter()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(limit)
        .map(|line| parse_runtime_log_line(line, source))
        .collect::<Vec<_>>();

    items.sort_by_key(|entry| Reverse(entry.timestamp));
    Ok(items)
}

fn parse_runtime_log_line(line: String, source: &str) -> LogEntry {
    let masked = mask_sensitive_segments(&line);
    let timestamp = parse_runtime_timestamp(&masked).unwrap_or_else(Utc::now);
    let level = parse_runtime_level(&masked);
    let message = strip_runtime_prefix(&masked);

    LogEntry {
        timestamp,
        level,
        source: source.to_string(),
        message,
    }
}

fn parse_runtime_timestamp(line: &str) -> Option<DateTime<Utc>> {
    let candidate = line.get(0..26)?;
    DateTime::parse_from_str(
        &format!("{candidate}+00:00"),
        "%Y-%m-%d %H:%M:%S%.6f%:z",
    )
    .ok()
    .map(|value| value.with_timezone(&Utc))
}

fn parse_runtime_level(line: &str) -> String {
    ["ERROR", "WARN", "INFO", "DEBUG", "TRACE"]
        .iter()
        .find(|level| line.contains(**level))
        .map(|level| (*level).to_string())
        .unwrap_or_else(|| "INFO".to_string())
}

fn build_log_stream_filter(
    query: &LogsQueryParams,
) -> Result<LogStreamFilter, WebError> {
    if matches!(
        query.source.as_deref(),
        Some(value) if value != "stream" && value != "runtime" && value != "audit"
    ) {
        return Err(WebError::invalid_input(
            "source",
            "Log source must be stream, runtime or audit.",
        ));
    }

    Ok(LogStreamFilter {
        source: query.source.clone(),
        level: query.level.clone(),
    })
}

fn normalize_log_entries(
    items: &mut Vec<LogEntry>,
    filter: &LogStreamFilter,
    limit: usize,
) {
    let mut seen = HashSet::new();
    items.retain(|entry| {
        filter.matches(entry)
            && seen.insert(format!(
                "{}|{}|{}|{}",
                entry.timestamp.to_rfc3339(),
                entry.level,
                entry.source,
                entry.message
            ))
    });
    items.sort_by_key(|entry| Reverse(entry.timestamp));
    items.truncate(limit);
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum LogStreamPayload {
    Replay { items: Vec<LogEntry> },
    Entry { item: LogEntry },
}

async fn handle_logs_socket(
    mut socket: WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>,
    mut receiver: broadcast::Receiver<LogEntry>,
    filter: LogStreamFilter,
    replay: Vec<LogEntry>,
) {
    if send_logs_payload(
        &mut socket,
        &LogStreamPayload::Replay { items: replay },
    )
    .await
    .is_err()
    {
        let _ = socket.close(None).await;
        return;
    }

    loop {
        tokio::select! {
            maybe_message = socket.next() => {
                match maybe_message {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            received = receiver.recv() => {
                match received {
                    Ok(entry) => {
                        if filter.matches(&entry)
                            && send_logs_payload(&mut socket, &LogStreamPayload::Entry { item: entry }).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    let _ = socket.close(None).await;
}

async fn send_logs_payload(
    socket: &mut WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>,
    payload: &LogStreamPayload,
) -> Result<(), WebError> {
    let body = serde_json::to_string(payload)
        .map_err(|error| WebError::internal(error.to_string()))?;
    socket
        .send(Message::Text(body.into()))
        .await
        .map_err(|error| WebError::internal(error.to_string()))
}

fn websocket_accept_key(headers: &HeaderMap) -> Result<String, WebError> {
    let connection = headers
        .get(header::CONNECTION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let upgrade = headers
        .get(header::UPGRADE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let version = headers
        .get("sec-websocket-version")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let key = headers
        .get("sec-websocket-key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            WebError::invalid_input(
                "sec-websocket-key",
                "Missing WebSocket handshake key.",
            )
        })?;

    if !connection.split(',').any(|part| part.trim() == "upgrade")
        || upgrade != "websocket"
        || version != "13"
    {
        return Err(WebError::invalid_input(
            "upgrade",
            "Invalid WebSocket upgrade request.",
        ));
    }

    Ok(derive_accept_key(key.as_bytes()))
}

fn strip_runtime_prefix(line: &str) -> String {
    let Some(rest) = line.get(26..) else {
        return line.to_string();
    };

    let rest = rest.trim_start();
    let Some((_, message)) = rest.split_once(char::is_whitespace) else {
        return line.to_string();
    };

    let message = message.trim_start();
    if message.is_empty() {
        line.to_string()
    } else {
        message.to_string()
    }
}

fn mask_sensitive_segments(line: &str) -> String {
    let mut masked = line.to_string();
    let keys = [
        "token",
        "access_token",
        "refresh_token",
        "password",
        "secret",
        "client_secret",
    ];

    for key in keys {
        masked = mask_key_value(&masked, key);
    }

    Privacy::sanitize_google_drive_internal_path_for_log(&masked)
}

fn mask_key_value(line: &str, key: &str) -> String {
    let pattern = format!("{key}=");
    let Some(start) = line.find(&pattern) else {
        return line.to_string();
    };

    let value_start = start + pattern.len();
    let value_end = line[value_start..]
        .find([' ', '&', ',', ';', '"'])
        .map(|offset| value_start + offset)
        .unwrap_or(line.len());

    let value = &line[value_start..value_end];
    let masked_value = Privacy::mask_google_drive_token(value);

    format!(
        "{}{}{}",
        &line[..value_start],
        masked_value,
        &line[value_end..]
    )
}

#[derive(Debug, Deserialize)]
pub struct LogsQueryParams {
    pub source: Option<String>,
    pub level: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use chrono::Utc;
    use futures_util::StreamExt;
    use serde_json::Value;
    use tempfile::tempdir;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{
        connect_async, tungstenite::client::IntoClientRequest,
    };
    use tower::util::ServiceExt;

    use super::{
        mask_sensitive_segments, parse_runtime_level, read_named_logs,
        strip_runtime_prefix,
    };
    use crate::log_stream::LogStreamHub;
    use crate::web::{
        api::WebAppState,
        app::WebRuntimeConfig,
        auth::{self, SESSION_COOKIE_NAME},
        contracts::{LogEntry, SessionUser, UserRole},
        db::Database,
        drafts,
    };

    #[test]
    fn runtime_log_reader_returns_newest_first() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("runtime.log");
        fs::write(
            &log_path,
            concat!(
                "2026-04-20 10:00:00.000001 INFO first\n",
                "2026-04-20 10:00:01.000001 WARN second\n"
            ),
        )
        .expect("write log");

        let items = read_named_logs(tempdir.path(), 10, "stream")
            .expect("runtime logs");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].level, "WARN");
        assert_eq!(items[1].level, "INFO");
        assert_eq!(items[0].source, "stream");
    }

    #[test]
    fn read_named_logs_accepts_rotated_stream_log_without_log_extension() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("stream.2026-04-22");
        fs::write(
            &log_path,
            "2026-04-22 20:00:08.639730 INFO stream smoke_test\n",
        )
        .expect("write log");

        let entries =
            read_named_logs(tempdir.path(), 20, "stream").expect("read logs");

        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.contains("smoke_test"));
    }

    #[test]
    fn read_named_logs_accepts_rotated_stream_log_with_date_only_file_name() {
        let tempdir = tempdir().expect("tempdir");
        let log_path = tempdir.path().join("2026-04-22");
        fs::write(
            &log_path,
            "2026-04-22 20:00:08.639730 INFO stream bare_date_smoke_test\n",
        )
        .expect("write log");

        let entries =
            read_named_logs(tempdir.path(), 20, "stream").expect("read logs");

        assert_eq!(entries.len(), 1);
        assert!(entries[0].message.contains("bare_date_smoke_test"));
    }

    #[test]
    fn runtime_log_masking_hides_secret_values() {
        let masked = mask_sensitive_segments(
            "token=BearerAccessToken1234 password=my-secret",
        );

        assert!(masked.contains("Bear...1234") || masked.contains("Bear"));
        assert!(!masked.contains("my-secret"));
    }

    #[test]
    fn runtime_log_level_falls_back_to_info() {
        assert_eq!(parse_runtime_level("plain line"), "INFO");
    }

    #[test]
    fn runtime_log_message_strips_timestamp_and_level_prefix() {
        assert_eq!(
            strip_runtime_prefix(
                "2026-04-23 15:10:55.057307 INFO [INIT] Initializing EmbyStream..."
            ),
            "[INIT] Initializing EmbyStream..."
        );
    }

    async fn build_logs_test_router()
    -> (Router, Database, tempfile::TempDir, LogStreamHub) {
        let tempdir = tempdir().expect("tempdir");
        let data_dir = tempdir.path().join("web-data");
        let runtime_log_dir = tempdir.path().join("runtime-logs");
        let stream_log_dir = tempdir.path().join("stream-logs");
        fs::create_dir_all(&runtime_log_dir).expect("runtime logs dir");
        fs::create_dir_all(&stream_log_dir).expect("stream logs dir");

        let db = Database::new(data_dir.clone());
        db.initialize().await.expect("initialize db");
        let live_logs = LogStreamHub::new(64, 64);

        let state = WebAppState::new_with_live_logs(
            db.clone(),
            WebRuntimeConfig {
                listen: "127.0.0.1:17172".parse().expect("socket addr"),
                data_dir,
                tmdb_api_key: None,
                runtime_log_dir,
                stream_log_dir,
                executable_path: tempdir.path().join("embystream"),
                main_config_path: Some(tempdir.path().join("config.toml")),
            },
            live_logs.clone(),
        );

        let router = Router::new()
            .nest("/api/auth", auth::routes())
            .nest("/api/drafts", drafts::routes())
            .nest("/api/logs", super::routes())
            .with_state(state);

        (router, db, tempdir, live_logs)
    }

    async fn admin_session_cookie(db: &Database) -> String {
        let admin = db
            .find_user_by_login("admin".to_string())
            .await
            .expect("find admin")
            .expect("admin exists");
        let session_id = db
            .create_session(
                SessionUser {
                    id: admin.id,
                    username: admin.username,
                    email: admin.email,
                    role: UserRole::Admin,
                },
                None,
                None,
            )
            .await
            .expect("create session");
        format!("{SESSION_COOKIE_NAME}={session_id}")
    }

    async fn spawn_logs_server(router: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("serve test logs router");
        });
        format!("ws://{address}/api/logs/stream")
    }

    #[tokio::test]
    async fn logs_http_requires_admin() {
        let (router, _db, _tempdir, _live_logs) =
            build_logs_test_router().await;

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/logs")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn logs_http_replay_reads_recent_runtime_entries() {
        let (router, db, tempdir, _live_logs) = build_logs_test_router().await;
        let cookie = admin_session_cookie(&db).await;

        fs::write(
            tempdir.path().join("runtime-logs/runtime.log"),
            "2026-04-23 20:00:08.639730 INFO [WEB] runtime replay smoke\n",
        )
        .expect("write runtime log");

        let response = router
            .oneshot(
                Request::builder()
                    .uri("/api/logs?source=runtime&limit=10")
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let body: Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(body["items"].as_array().map(Vec::len), Some(1));
    }

    #[tokio::test]
    async fn logs_websocket_replays_recent_live_entries_for_admin() {
        let (router, db, _tempdir, live_logs) = build_logs_test_router().await;
        let cookie = admin_session_cookie(&db).await;
        let url = spawn_logs_server(router).await;

        let unique_message = format!(
            "websocket replay {}",
            Utc::now()
                .timestamp_nanos_opt()
                .expect("nanosecond timestamp")
        );
        live_logs.publish(LogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            source: "runtime".to_string(),
            message: unique_message.clone(),
        });

        let mut request = url.into_client_request().expect("client request");
        request
            .headers_mut()
            .insert(header::COOKIE, cookie.parse().expect("cookie header"));

        let (mut socket, _) = connect_async(request).await.expect("ws connect");
        let message = socket
            .next()
            .await
            .expect("ws replay frame")
            .expect("ws replay result");

        let text = message.to_text().expect("text frame");
        let payload: Value = serde_json::from_str(text).expect("json payload");
        assert_eq!(payload["kind"], "replay");
        let items = payload["items"].as_array().expect("replay items");
        assert!(
            items.iter().any(|item| item["message"] == unique_message),
            "replay should contain the recently published log entry"
        );
    }
}
