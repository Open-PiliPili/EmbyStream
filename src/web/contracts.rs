use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::{
    backend::{Backend, BackendNode},
    frontend::Frontend,
    general::{Emby, Log, StreamMode, UserAgent},
    http2::Http2,
    types::FallbackConfig,
};
use crate::oauthutil::OAuthToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WizardStreamMode {
    Frontend,
    Backend,
    Dual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftStatus {
    Draft,
    Generated,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    ConfigToml,
    NginxConf,
    DockerCompose,
    SystemdService,
    Pm2Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackgroundProvider {
    Tmdb,
    Bing,
    StaticFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: UserRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserAdminSummary {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub role: UserRole,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: SessionUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub login: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogoutResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftSummary {
    pub id: String,
    pub name: String,
    pub status: DraftStatus,
    pub stream_mode: WizardStreamMode,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftDocument {
    pub id: String,
    pub name: String,
    pub status: DraftStatus,
    pub stream_mode: WizardStreamMode,
    pub payload: WizardPayload,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftEnvelope {
    pub draft: DraftSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftDocumentEnvelope {
    pub draft: DraftDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardTemplateResponse {
    pub payload: WizardPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftListResponse {
    pub items: Vec<DraftSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDraftRequest {
    pub name: String,
    pub stream_mode: WizardStreamMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveDraftRequest {
    pub name: String,
    pub payload: WizardPayload,
    pub client_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveDraftResponse {
    pub draft: DraftRevision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftRevision {
    pub id: String,
    pub updated_at: DateTime<Utc>,
    pub server_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSetSummary {
    pub id: String,
    pub name: String,
    pub stream_mode: WizardStreamMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSetListResponse {
    pub items: Vec<ConfigSetSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSetEnvelope {
    pub config_set: ConfigSetSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
    pub artifact_type: ArtifactType,
    pub file_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDocument {
    pub artifact_type: ArtifactType,
    pub file_name: String,
    pub language: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactListResponse {
    pub items: Vec<ArtifactDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerateDraftResponse {
    pub config_set: ConfigSetSummary,
    pub artifacts: Vec<ArtifactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundItem {
    pub image_url: String,
    pub title: String,
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoginBackgroundResponse {
    pub provider: BackgroundProvider,
    pub fetched_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub items: Vec<BackgroundItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogListResponse {
    pub items: Vec<LogEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserListResponse {
    pub items: Vec<UserAdminSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserEnvelope {
    pub user: UserAdminSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: UserRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateUserDisabledRequest {
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateUserPasswordRequest {
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeOwnPasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMetricsResponse {
    pub cpu_usage_percent: f64,
    pub cpu_core_count: u32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_usage_percent: f64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_usage_percent: f64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataUpdateRequest {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WizardSharedGeneral {
    pub memory_mode: String,
    pub encipher_key: String,
    pub encipher_iv: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardSharedPayload {
    pub log: Log,
    pub general: WizardSharedGeneral,
    pub emby: Emby,
    pub user_agent: UserAgent,
    pub fallback: FallbackConfig,
    pub http2: Http2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardPayload {
    pub stream_mode: WizardStreamMode,
    pub shared: WizardSharedPayload,
    pub frontend: Option<Frontend>,
    pub backend: Option<Backend>,
    #[serde(default)]
    pub backend_nodes: Vec<BackendNode>,
    #[serde(default)]
    pub nginx: WizardNginxPayload,
    #[serde(default)]
    pub deployment: WizardDeploymentPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WizardNginxPayload {
    #[serde(default)]
    pub frontend: WizardFrontendNginxPayload,
    #[serde(default)]
    pub backend: WizardBackendNginxPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WizardDeploymentPayload {
    #[serde(default)]
    pub systemd: WizardSystemdPayload,
    #[serde(default)]
    pub pm2: WizardPm2Payload,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WizardSystemdPayload {
    #[serde(default)]
    pub binary_path: String,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub config_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WizardPm2Payload {
    #[serde(default)]
    pub binary_path: String,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub config_path: String,
    #[serde(default)]
    pub out_file: String,
    #[serde(default)]
    pub error_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardFrontendNginxPayload {
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub ssl_certificate: String,
    #[serde(default)]
    pub ssl_certificate_key: String,
    #[serde(default = "default_frontend_client_max_body_size")]
    pub client_max_body_size: String,
    #[serde(default = "default_frontend_static_pattern")]
    pub static_location_pattern: String,
    #[serde(default = "default_frontend_websocket_pattern")]
    pub websocket_location_pattern: String,
}

impl Default for WizardFrontendNginxPayload {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            ssl_certificate: String::new(),
            ssl_certificate_key: String::new(),
            client_max_body_size: default_frontend_client_max_body_size(),
            static_location_pattern: default_frontend_static_pattern(),
            websocket_location_pattern: default_frontend_websocket_pattern(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardBackendNginxPayload {
    #[serde(default)]
    pub server_name: String,
    #[serde(default)]
    pub ssl_certificate: String,
    #[serde(default)]
    pub ssl_certificate_key: String,
    #[serde(default = "default_backend_client_max_body_size")]
    pub client_max_body_size: String,
    #[serde(default = "default_backend_resolver_provider")]
    pub resolver_provider: String,
    #[serde(default)]
    pub custom_resolvers: String,
    #[serde(default = "default_backend_access_log")]
    pub access_log_path: String,
    #[serde(default = "default_backend_error_log")]
    pub error_log_path: String,
    #[serde(default = "default_backend_google_access_log")]
    pub google_drive_access_log_path: String,
}

impl Default for WizardBackendNginxPayload {
    fn default() -> Self {
        Self {
            server_name: String::new(),
            ssl_certificate: String::new(),
            ssl_certificate_key: String::new(),
            client_max_body_size: default_backend_client_max_body_size(),
            resolver_provider: default_backend_resolver_provider(),
            custom_resolvers: String::new(),
            access_log_path: default_backend_access_log(),
            error_log_path: default_backend_error_log(),
            google_drive_access_log_path: default_backend_google_access_log(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardGoogleDriveTokenPayload {
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default = "default_google_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub expiry: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for WizardGoogleDriveTokenPayload {
    fn default() -> Self {
        Self {
            access_token: String::new(),
            refresh_token: String::new(),
            token_type: default_google_token_type(),
            expiry: None,
        }
    }
}

fn default_google_token_type() -> String {
    "Bearer".to_string()
}

fn default_frontend_client_max_body_size() -> String {
    "100M".to_string()
}

fn default_frontend_static_pattern() -> String {
    r"\.(webp|jpg|jpeg|png|gif|ico|css|js|html)$|Images|fonts".to_string()
}

fn default_frontend_websocket_pattern() -> String {
    r"/(socket|embywebsocket)".to_string()
}

fn default_backend_client_max_body_size() -> String {
    "1G".to_string()
}

fn default_backend_resolver_provider() -> String {
    "none".to_string()
}

fn default_backend_access_log() -> String {
    "/var/log/nginx/embystream_access.log".to_string()
}

fn default_backend_error_log() -> String {
    "/var/log/nginx/embystream_error.log".to_string()
}

fn default_backend_google_access_log() -> String {
    "/var/log/nginx/google_drive_access.log".to_string()
}

pub fn wizard_token_payload_from_oauth(
    token: Option<&OAuthToken>,
) -> Option<WizardGoogleDriveTokenPayload> {
    token.map(|token| WizardGoogleDriveTokenPayload {
        access_token: token.access_token.clone(),
        refresh_token: token.refresh_token.clone(),
        token_type: token.token_type.clone(),
        expiry: token.expiry,
    })
}

pub fn wizard_token_payload_into_oauth(
    token: Option<&WizardGoogleDriveTokenPayload>,
) -> Option<OAuthToken> {
    token.and_then(|token| {
        if token.access_token.trim().is_empty()
            && token.refresh_token.trim().is_empty()
        {
            return None;
        }

        Some(OAuthToken {
            access_token: token.access_token.clone(),
            refresh_token: token.refresh_token.clone(),
            token_type: if token.token_type.trim().is_empty() {
                default_google_token_type()
            } else {
                token.token_type.clone()
            },
            expiry: token.expiry,
        })
    })
}

impl From<StreamMode> for WizardStreamMode {
    fn from(value: StreamMode) -> Self {
        match value {
            StreamMode::Frontend => Self::Frontend,
            StreamMode::Backend => Self::Backend,
            StreamMode::Dual => Self::Dual,
        }
    }
}

impl From<WizardStreamMode> for StreamMode {
    fn from(value: WizardStreamMode) -> Self {
        match value {
            WizardStreamMode::Frontend => Self::Frontend,
            WizardStreamMode::Backend => Self::Backend,
            WizardStreamMode::Dual => Self::Dual,
        }
    }
}
