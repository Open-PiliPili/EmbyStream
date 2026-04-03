use std::sync::Arc;

use dashmap::DashMap;
use hyper::{HeaderMap, header};
use tokio::sync::Mutex as AsyncMutex;

use crate::{
    AppState, config::backend::BackendNode,
    config::core::persist_google_drive_access_token, debug_log, error_log,
    info_log,
};

use super::google_drive::BACKEND_TYPE as GOOGLE_DRIVE_BACKEND_TYPE;

const GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN: &str = "GOOGLE-DRIVE-AUTH";

pub type GoogleDriveRefreshLocks = DashMap<String, Arc<AsyncMutex<()>>>;

fn cache_key(node_uuid: &str) -> String {
    format!(
        "google-drive-token:{}",
        node_uuid.trim().to_ascii_lowercase()
    )
}

pub fn is_google_drive_node(node: &BackendNode) -> bool {
    node.backend_type
        .eq_ignore_ascii_case(GOOGLE_DRIVE_BACKEND_TYPE)
}

pub fn current_access_token(
    cache: &DashMap<String, String>,
    node: &BackendNode,
) -> Option<String> {
    let cfg = node.google_drive.as_ref()?;
    let key = cache_key(&cfg.node_uuid);
    if let Some(token) = cache.get(&key) {
        let token = token.value().trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }
    let token = cfg.access_token.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

pub fn authorization_line_for_remote(
    cache: &DashMap<String, String>,
    node: &BackendNode,
) -> Option<String> {
    current_access_token(cache, node).map(|token| format!("Bearer {token}"))
}

pub fn extra_headers_from_auth_line(
    line: &str,
) -> Result<HeaderMap, &'static str> {
    let mut map = HeaderMap::new();
    let value = line
        .parse()
        .map_err(|_| "invalid authorization header value")?;
    map.insert(header::AUTHORIZATION, value);
    Ok(map)
}

pub fn accel_header_name() -> header::HeaderName {
    header::HeaderName::from_static("x-embystream-upstream-authorization")
}

pub async fn refresh_access_token_now(
    state: Arc<AppState>,
    node: &BackendNode,
) -> Result<String, ()> {
    let Some(cfg) = node.google_drive.as_ref() else {
        return Err(());
    };
    let node_uuid = cfg.node_uuid.trim();
    if node_uuid.is_empty() {
        return Err(());
    }

    let key = cache_key(node_uuid);
    let lock = state
        .google_drive_refresh_locks
        .entry(key.clone())
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone();
    let _guard = lock.lock().await;

    let client = state.get_google_drive_client().await.clone();
    let refreshed = client
        .refresh_access_token(
            &cfg.client_id,
            &cfg.client_secret,
            &cfg.refresh_token,
        )
        .await
        .map_err(|error| {
            error_log!(
                GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                "google_drive_refresh_failed node={} error={}",
                node.name,
                error
            );
        })?;

    state
        .google_drive_access_token_cache
        .insert(key, refreshed.access_token.clone());

    {
        let mut config = state.config.write().await;
        for backend_node in &mut config.backend_nodes {
            let Some(google_drive) = backend_node.google_drive.as_mut() else {
                continue;
            };
            if google_drive.node_uuid.trim() != node_uuid {
                continue;
            }
            google_drive.access_token = refreshed.access_token.clone();
            break;
        }
        let _config_write_guard = state.config_write_lock.lock().await;
        if let Err(error) = persist_google_drive_access_token(
            &config.path,
            node_uuid,
            &refreshed.access_token,
        ) {
            error_log!(
                GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
                "google_drive_refresh_persist_failed node={} error={}",
                node.name,
                error
            );
        }
    }

    info_log!(
        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
        "google_drive_refresh_succeeded node={}",
        node.name
    );
    Ok(refreshed.access_token)
}

pub fn trigger_refresh_if_needed(state: Arc<AppState>, node: BackendNode) {
    let Some(cfg) = node.google_drive.as_ref() else {
        return;
    };
    let node_uuid = cfg.node_uuid.trim();
    if node_uuid.is_empty() {
        return;
    }

    let key = cache_key(node_uuid);
    let lock = state
        .google_drive_refresh_locks
        .entry(key)
        .or_insert_with(|| Arc::new(AsyncMutex::new(())))
        .clone();

    let Ok(refresh_guard) = lock.try_lock_owned() else {
        debug_log!(
            GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
            "google_drive_refresh_skip node={} reason=refresh_already_running",
            node.name
        );
        return;
    };

    tokio::spawn(async move {
        let _refresh_guard = refresh_guard;
        let _ = refresh_access_token_now(state, &node).await;
    });
}
