use std::{collections::VecDeque, sync::Arc};

use reqwest::{
    Client as HttpClient,
    header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    client::BuildableClient, core::backend::google_drive::DriveLookup,
    network::NetworkPlugin,
};

const GOOGLE_DRIVE_API_BASE: &str = "https://www.googleapis.com/drive/v3";
const GOOGLE_OAUTH_TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_DRIVE_FOLDER_MIME: &str = "application/vnd.google-apps.folder";

#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    api_base: Arc<str>,
    oauth_token_endpoint: Arc<str>,
}

#[derive(Debug, Error)]
pub enum GoogleDriveApiError {
    #[error("googleDrive access_token is empty")]
    EmptyAccessToken,
    #[error("googleDrive refresh_token is empty")]
    EmptyRefreshToken,
    #[error("googleDrive client_id is empty")]
    EmptyClientId,
    #[error("googleDrive file path is empty")]
    EmptyPath,
    #[error("googleDrive shared drive name is empty")]
    EmptyDriveName,
    #[error("googleDrive shared drive '{0}' was not found")]
    DriveNotFound(String),
    #[error("googleDrive shared drive name '{0}' matched multiple drives")]
    AmbiguousDriveName(String),
    #[error(
        "googleDrive path component '{component}' in '{logical_path}' matched multiple entries"
    )]
    AmbiguousPathComponent {
        component: String,
        logical_path: String,
    },
    #[error("googleDrive API returned status {status}: {body}")]
    ApiStatus { status: u16, body: String },
    #[error("googleDrive HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleTokenRefreshResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedDriveRef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoogleDriveResolvedFile {
    pub drive_id: String,
    pub file_id: String,
}

#[derive(Debug, Deserialize)]
struct TokenRefreshPayload {
    access_token: String,
    #[serde(default)]
    token_type: String,
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct DrivesListResponse {
    #[serde(default)]
    drives: Vec<SharedDriveDto>,
}

#[derive(Debug, Deserialize)]
struct SharedDriveDto {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct FilesListResponse {
    #[serde(default)]
    files: Vec<FileEntryDto>,
}

#[derive(Debug, Deserialize)]
struct FileEntryDto {
    id: String,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl BuildableClient for Client {
    fn build_from_plugins(_plugins: Vec<Box<dyn NetworkPlugin>>) -> Self {
        Self::new()
    }
}

impl Client {
    pub fn new() -> Self {
        Self::with_endpoints(GOOGLE_DRIVE_API_BASE, GOOGLE_OAUTH_TOKEN_ENDPOINT)
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(
        api_base: &str,
        oauth_token_endpoint: &str,
    ) -> Self {
        Self::with_endpoints(api_base, oauth_token_endpoint)
    }

    fn with_endpoints(api_base: &str, oauth_token_endpoint: &str) -> Self {
        let http = HttpClient::builder()
            .use_rustls_tls()
            .build()
            .unwrap_or_else(|_| HttpClient::new());
        Self {
            http,
            api_base: Arc::from(api_base.to_string()),
            oauth_token_endpoint: Arc::from(oauth_token_endpoint.to_string()),
        }
    }

    pub async fn refresh_access_token(
        &self,
        client_id: &str,
        client_secret: &str,
        refresh_token: &str,
    ) -> Result<GoogleTokenRefreshResponse, GoogleDriveApiError> {
        let client_id = client_id.trim();
        if client_id.is_empty() {
            return Err(GoogleDriveApiError::EmptyClientId);
        }

        let refresh_token = refresh_token.trim();
        if refresh_token.is_empty() {
            return Err(GoogleDriveApiError::EmptyRefreshToken);
        }

        let response = self
            .http
            .post(self.oauth_token_endpoint.as_ref())
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&[
                ("client_id", client_id),
                ("client_secret", client_secret.trim()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(GoogleDriveApiError::ApiStatus { status, body });
        }

        let payload: TokenRefreshPayload = response.json().await?;
        Ok(GoogleTokenRefreshResponse {
            access_token: payload.access_token,
            token_type: payload.token_type,
            expires_in: payload.expires_in,
        })
    }

    pub async fn find_shared_drive_by_name(
        &self,
        access_token: &str,
        drive_name: &str,
    ) -> Result<Option<SharedDriveRef>, GoogleDriveApiError> {
        let access_token = access_token.trim();
        if access_token.is_empty() {
            return Err(GoogleDriveApiError::EmptyAccessToken);
        }

        let drive_name = drive_name.trim();
        if drive_name.is_empty() {
            return Err(GoogleDriveApiError::EmptyDriveName);
        }

        let response = self
            .http
            .get(format!("{}/drives", self.api_base))
            .header(AUTHORIZATION, bearer_token(access_token))
            .query(&[
                ("pageSize", "2"),
                ("useDomainAdminAccess", "false"),
                (
                    "q",
                    &format!("name = '{}'", escape_query_literal(drive_name)),
                ),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(GoogleDriveApiError::ApiStatus { status, body });
        }

        let payload: DrivesListResponse = response.json().await?;
        match payload.drives.len() {
            0 => Ok(None),
            1 => {
                let drive = &payload.drives[0];
                Ok(Some(SharedDriveRef {
                    id: drive.id.clone(),
                    name: drive.name.clone(),
                }))
            }
            _ => Err(GoogleDriveApiError::AmbiguousDriveName(
                drive_name.to_string(),
            )),
        }
    }

    pub async fn resolve_file_id_by_path(
        &self,
        access_token: &str,
        lookup: &DriveLookup,
        relative_path: &str,
    ) -> Result<GoogleDriveResolvedFile, GoogleDriveApiError> {
        let access_token = access_token.trim();
        if access_token.is_empty() {
            return Err(GoogleDriveApiError::EmptyAccessToken);
        }

        let mut segments = normalize_non_empty_path(relative_path);
        if segments.is_empty() {
            return Err(GoogleDriveApiError::EmptyPath);
        }

        let drive = match lookup {
            DriveLookup::DriveId(id) => SharedDriveRef {
                id: id.trim().to_string(),
                name: String::new(),
            },
            DriveLookup::DriveName(name) => self
                .find_shared_drive_by_name(access_token, name)
                .await?
                .ok_or_else(|| {
                    GoogleDriveApiError::DriveNotFound(name.to_string())
                })?,
        };

        let mut parent_id = drive.id.clone();
        let mut current_file_id = drive.id.clone();
        let mut logical_segments = VecDeque::with_capacity(segments.len());

        while let Some(segment) = segments.pop_front() {
            logical_segments.push_back(segment.to_string());
            let is_last = segments.is_empty();
            let q = build_files_query(segment, &parent_id, is_last);
            let response = self
                .http
                .get(format!("{}/files", self.api_base))
                .header(AUTHORIZATION, bearer_token(access_token))
                .query(&[
                    ("pageSize", "2"),
                    ("driveId", drive.id.as_str()),
                    ("corpora", "drive"),
                    ("includeItemsFromAllDrives", "true"),
                    ("supportsAllDrives", "true"),
                    ("fields", "files(id)"),
                    ("q", q.as_str()),
                ])
                .send()
                .await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();
                return Err(GoogleDriveApiError::ApiStatus { status, body });
            }

            let payload: FilesListResponse = response.json().await?;
            match payload.files.len() {
                0 => {
                    return Err(GoogleDriveApiError::DriveNotFound(format!(
                        "/{}",
                        logical_segments.make_contiguous().join("/")
                    )));
                }
                1 => {
                    current_file_id = payload.files[0].id.clone();
                    parent_id = current_file_id.clone();
                }
                _ => {
                    return Err(GoogleDriveApiError::AmbiguousPathComponent {
                        component: segment.to_string(),
                        logical_path: format!(
                            "/{}",
                            logical_segments.make_contiguous().join("/")
                        ),
                    });
                }
            }
        }

        Ok(GoogleDriveResolvedFile {
            drive_id: drive.id,
            file_id: current_file_id,
        })
    }

    pub fn build_media_url(&self, file_id: &str) -> String {
        format!(
            "{}/files/{}?alt=media&supportsAllDrives=true&acknowledgeAbuse=true",
            self.api_base.trim_end_matches('/'),
            file_id.trim()
        )
    }

    pub fn build_media_url_with_token(
        &self,
        file_id: &str,
        access_token: &str,
    ) -> String {
        format!(
            "{}/files/{}?alt=media&supportsAllDrives=true&acknowledgeAbuse=true&access_token={}",
            self.api_base.trim_end_matches('/'),
            file_id.trim(),
            access_token.trim()
        )
    }
}

fn normalize_non_empty_path(path: &str) -> VecDeque<&str> {
    path.split(['/', '\\'])
        .filter(|segment| !segment.trim().is_empty())
        .collect()
}

fn escape_query_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn bearer_token(access_token: &str) -> String {
    format!("Bearer {}", access_token.trim())
}

fn build_files_query(segment: &str, parent_id: &str, is_last: bool) -> String {
    let escaped_segment = escape_query_literal(segment);
    let escaped_parent_id = escape_query_literal(parent_id);
    if is_last {
        format!(
            "name = '{}' and '{}' in parents and trashed = false",
            escaped_segment, escaped_parent_id
        )
    } else {
        format!(
            "name = '{}' and '{}' in parents and mimeType = '{}' and trashed = false",
            escaped_segment, escaped_parent_id, GOOGLE_DRIVE_FOLDER_MIME
        )
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{
        Arc, Once,
        atomic::{AtomicUsize, Ordering},
    };

    use rustls::crypto::aws_lc_rs;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    static RUSTLS_CRYPTO_INIT: Once = Once::new();

    fn ensure_rustls_crypto_provider() {
        RUSTLS_CRYPTO_INIT.call_once(|| {
            let _ = aws_lc_rs::default_provider().install_default();
        });
    }

    #[test]
    fn build_media_url_adds_required_google_params() {
        let client = Client::with_endpoints(
            "https://www.googleapis.com/drive/v3",
            GOOGLE_OAUTH_TOKEN_ENDPOINT,
        );
        let url = client.build_media_url("file-123");
        assert_eq!(
            url,
            "https://www.googleapis.com/drive/v3/files/file-123?alt=media&supportsAllDrives=true&acknowledgeAbuse=true"
        );
    }

    #[test]
    fn build_media_url_with_token_appends_access_token() {
        let client = Client::with_endpoints(
            "https://www.googleapis.com/drive/v3",
            GOOGLE_OAUTH_TOKEN_ENDPOINT,
        );
        let url = client.build_media_url_with_token("file-123", "ya29.token");
        assert_eq!(
            url,
            "https://www.googleapis.com/drive/v3/files/file-123?alt=media&supportsAllDrives=true&acknowledgeAbuse=true&access_token=ya29.token"
        );
    }

    #[test]
    fn build_files_query_uses_folder_filter_for_non_terminal_segments() {
        let q = build_files_query("电视剧", "drive-123", false);
        assert!(q.contains("mimeType = 'application/vnd.google-apps.folder'"));
        assert!(q.contains("'drive-123' in parents"));
        assert!(q.contains("trashed = false"));
    }

    #[tokio::test]
    async fn refresh_access_token_posts_expected_form() {
        ensure_rustls_crypto_provider();

        let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let seen_clone = seen.clone();
        let base = spawn_mock_server(vec![Box::new(move |request| {
            let seen_clone = seen_clone.clone();
            Box::pin(async move {
                seen_clone.lock().await.push(request.clone());
                http_response(
                    200,
                    "application/json",
                    r#"{"access_token":"new-token","token_type":"Bearer","expires_in":3600}"#,
                )
            })
        })])
        .await;

        let client =
            Client::with_endpoints("http://unused", &format!("{base}/token"));
        let refreshed = client
            .refresh_access_token("client-id", "secret", "refresh-1")
            .await
            .expect("refresh token");

        assert_eq!(refreshed.access_token, "new-token");
        assert_eq!(refreshed.token_type, "Bearer");
        assert_eq!(refreshed.expires_in, Some(3600));

        let requests = seen.lock().await;
        let req = requests.first().expect("captured request");
        assert!(req.starts_with("POST /token HTTP/1.1"));
        assert!(
            req.contains("content-type: application/x-www-form-urlencoded")
        );
        assert!(req.contains(
            "client_id=client-id&client_secret=secret&refresh_token=refresh-1&grant_type=refresh_token"
        ));
    }

    #[tokio::test]
    async fn find_shared_drive_by_name_uses_exact_query() {
        ensure_rustls_crypto_provider();

        let seen = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let seen_clone = seen.clone();
        let base = spawn_mock_server(vec![Box::new(move |request| {
            let seen_clone = seen_clone.clone();
            Box::pin(async move {
                seen_clone.lock().await.push(request.clone());
                http_response(
                    200,
                    "application/json",
                    r#"{"drives":[{"id":"drive-123","name":"pilipili"}]}"#,
                )
            })
        })])
        .await;

        let client = Client::with_endpoints(
            &format!("{base}/drive/v3"),
            "http://unused",
        );
        let drive = client
            .find_shared_drive_by_name("access-token", "pilipili")
            .await
            .expect("find drive")
            .expect("drive result");

        assert_eq!(drive.id, "drive-123");
        assert_eq!(drive.name, "pilipili");

        let requests = seen.lock().await;
        let req = requests.first().expect("captured request");
        assert!(req.starts_with("GET /drive/v3/drives?"));
        assert!(req.contains("q=name+%3D+%27pilipili%27"));
        assert!(req.contains("useDomainAdminAccess=false"));
        assert!(req.contains("authorization: Bearer access-token"));
    }

    #[tokio::test]
    async fn resolve_file_id_by_path_walks_shared_drive_segments() {
        ensure_rustls_crypto_provider();

        let hit = Arc::new(AtomicUsize::new(0));
        let hit_clone = hit.clone();
        let handlers: Vec<Handler> = (0..3)
            .map(|_| {
                let hit_clone = hit_clone.clone();
                Box::new(move |request: String| -> Pin<Box<dyn Future<Output = String> + Send>> {
                    let hit_clone = hit_clone.clone();
                    Box::pin(async move {
                        let step = hit_clone.fetch_add(1, Ordering::SeqCst);
                        match step {
                            0 => {
                                assert!(request.starts_with("GET /drive/v3/files?"));
                                assert!(request.contains("driveId=drive-123"));
                                assert!(request.contains("q=name+%3D+%27%E7%94%B5%E5%BD%B1%27"));
                                assert!(request.contains("%27drive-123%27+in+parents"));
                                assert!(request.contains(
                                    "mimeType+%3D+%27application%2Fvnd.google-apps.folder%27"
                                ));
                                http_response(
                                    200,
                                    "application/json",
                                    r#"{"files":[{"id":"folder-1"}]}"#,
                                )
                            }
                            1 => {
                                assert!(request.contains("q=name+%3D+%272026%27"));
                                assert!(request.contains("%27folder-1%27+in+parents"));
                                http_response(
                                    200,
                                    "application/json",
                                    r#"{"files":[{"id":"folder-2"}]}"#,
                                )
                            }
                            _ => {
                                assert!(request.contains("q=name+%3D+%27test.mkv%27"));
                                assert!(request.contains("%27folder-2%27+in+parents"));
                                http_response(
                                    200,
                                    "application/json",
                                    r#"{"files":[{"id":"file-789"}]}"#,
                                )
                            }
                        }
                    })
                }) as Handler
            })
            .collect();
        let base = spawn_mock_server(handlers).await;

        let client = Client::with_endpoints(
            &format!("{base}/drive/v3"),
            "http://unused",
        );
        let resolved = client
            .resolve_file_id_by_path(
                "access-token",
                &DriveLookup::DriveId("drive-123".into()),
                "/电影/2026/test.mkv",
            )
            .await
            .expect("resolve file");

        assert_eq!(resolved.drive_id, "drive-123");
        assert_eq!(resolved.file_id, "file-789");
    }

    type Handler = Box<
        dyn Fn(String) -> Pin<Box<dyn Future<Output = String> + Send>>
            + Send
            + Sync,
    >;

    async fn spawn_mock_server(handlers: Vec<Handler>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");

        tokio::spawn(async move {
            for handler in handlers {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let request = read_http_request(&mut stream).await;
                let response = handler(request).await;
                stream.write_all(response.as_bytes()).await.expect("write");
            }
        });

        format!("http://{}", addr)
    }

    fn http_response(status: u16, content_type: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status} OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }

    async fn read_http_request(stream: &mut tokio::net::TcpStream) -> String {
        let mut buffer = Vec::with_capacity(8192);
        let mut chunk = [0_u8; 2048];
        let mut expected_len = None;

        loop {
            let n = stream.read(&mut chunk).await.expect("read");
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..n]);

            if expected_len.is_none() {
                expected_len = content_length_and_header_size(&buffer);
            }

            if let Some((content_length, header_size)) = expected_len {
                if buffer.len() >= header_size + content_length {
                    break;
                }
            }
        }

        String::from_utf8_lossy(&buffer).to_string()
    }

    fn content_length_and_header_size(buffer: &[u8]) -> Option<(usize, usize)> {
        let header_end = buffer
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|idx| idx + 4)?;
        let headers = String::from_utf8_lossy(&buffer[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if !name.eq_ignore_ascii_case("content-length") {
                    return None;
                }
                value.trim().parse::<usize>().ok()
            })
            .unwrap_or(0);
        Some((content_length, header_end))
    }
}
