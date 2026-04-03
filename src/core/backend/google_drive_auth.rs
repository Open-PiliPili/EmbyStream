use std::sync::Arc;

use dashmap::DashMap;
use hyper::{HeaderMap, header};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{Duration, interval};

use crate::{
    AppState,
    config::backend::{BackendNode, GoogleDriveConfig},
    config::core::persist_google_drive_access_token,
    debug_log, error_log, info_log,
};

use super::google_drive::BACKEND_TYPE as GOOGLE_DRIVE_BACKEND_TYPE;

const GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN: &str = "GOOGLE-DRIVE-AUTH";
const PERIODIC_REFRESH_INTERVAL_SECS: u64 = 45 * 60;

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

fn collect_refreshable_google_drive_nodes(
    nodes: &[BackendNode],
) -> Vec<BackendNode> {
    nodes
        .iter()
        .filter_map(|node| {
            let cfg = google_drive_config(node)?;
            if cfg.node_uuid.trim().is_empty() {
                return None;
            }
            if cfg.refresh_token.trim().is_empty() {
                return None;
            }
            Some(node.clone())
        })
        .collect()
}

fn google_drive_config(node: &BackendNode) -> Option<&GoogleDriveConfig> {
    if !is_google_drive_node(node) {
        return None;
    }
    node.google_drive.as_ref()
}

async fn refresh_all_google_drive_nodes(state: Arc<AppState>) {
    let nodes = {
        let config = state.get_config().await;
        collect_refreshable_google_drive_nodes(&config.backend_nodes)
    };

    if nodes.is_empty() {
        debug_log!(
            GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
            "google_drive_periodic_refresh_skip reason=no_nodes"
        );
        return;
    }

    info_log!(
        GOOGLE_DRIVE_AUTH_LOGGER_DOMAIN,
        "google_drive_periodic_refresh_tick nodes={}",
        nodes.len()
    );

    for node in nodes {
        trigger_refresh_if_needed(state.clone(), node);
    }
}

pub fn start_periodic_refresh_task(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut ticker =
            interval(Duration::from_secs(PERIODIC_REFRESH_INTERVAL_SECS));
        ticker.tick().await;

        loop {
            ticker.tick().await;
            refresh_all_google_drive_nodes(state.clone()).await;
        }
    });
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

#[cfg(test)]
mod tests {
    use super::collect_refreshable_google_drive_nodes;
    use crate::config::backend::{BackendNode, GoogleDriveConfig};

    fn google_drive_node(
        name: &str,
        node_uuid: &str,
        refresh_token: &str,
    ) -> BackendNode {
        BackendNode {
            name: name.to_string(),
            backend_type: "googleDrive".to_string(),
            pattern: String::new(),
            pattern_regex: None,
            base_url: String::new(),
            port: String::new(),
            path: String::new(),
            priority: 0,
            proxy_mode: "redirect".to_string(),
            client_speed_limit_kbs: 0,
            client_burst_speed_kbs: 0,
            path_rewrites: vec![],
            anti_reverse_proxy: Default::default(),
            path_rewriter_cache: vec![],
            uuid: String::new(),
            disk: None,
            open_list: None,
            direct_link: None,
            google_drive: Some(GoogleDriveConfig {
                node_uuid: node_uuid.to_string(),
                client_id: "client-id".to_string(),
                client_secret: "client-secret".to_string(),
                drive_id: String::new(),
                drive_name: String::new(),
                access_token: String::new(),
                refresh_token: refresh_token.to_string(),
            }),
            webdav: None,
        }
    }

    #[test]
    fn collect_refreshable_google_drive_nodes_only_keeps_valid_google_drive_nodes()
     {
        let nodes = vec![
            google_drive_node("google-1", "node-1", "refresh-1"),
            google_drive_node("google-missing-uuid", "", "refresh-2"),
            google_drive_node("google-missing-refresh", "node-3", ""),
            BackendNode {
                name: "disk-1".to_string(),
                backend_type: "disk".to_string(),
                pattern: String::new(),
                pattern_regex: None,
                base_url: String::new(),
                port: String::new(),
                path: String::new(),
                priority: 0,
                proxy_mode: "redirect".to_string(),
                client_speed_limit_kbs: 0,
                client_burst_speed_kbs: 0,
                path_rewrites: vec![],
                anti_reverse_proxy: Default::default(),
                path_rewriter_cache: vec![],
                uuid: String::new(),
                disk: None,
                open_list: None,
                direct_link: None,
                google_drive: None,
                webdav: None,
            },
        ];

        let refreshable = collect_refreshable_google_drive_nodes(&nodes);

        assert_eq!(refreshable.len(), 1);
        assert_eq!(refreshable[0].name, "google-1");
    }
}
