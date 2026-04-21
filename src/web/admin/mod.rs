use std::{
    collections::BTreeSet, fs, path::Path as FsPath, thread, time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{delete, get, patch},
};
use axum_extra::extract::CookieJar;
use serde_json::json;
use sysinfo::{
    CpuRefreshKind, Disks, MemoryRefreshKind, ProcessRefreshKind,
    ProcessesToUpdate, RefreshKind, System, get_current_pid,
};

use crate::web::{
    api::WebAppState,
    auth::{hash_password, session_user_from_jar},
    contracts::{
        LogoutResponse, SystemMetricsResponse, UpdateUserDisabledRequest,
        UpdateUserPasswordRequest, UpdateUserRoleRequest, UserEnvelope,
        UserListResponse, UserRole,
    },
    error::WebError,
};

pub fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/system", get(get_system_metrics))
        .route("/users", get(list_users))
        .route("/users/{user_id}/role", patch(update_user_role))
        .route("/users/{user_id}/disabled", patch(update_user_disabled))
        .route("/users/{user_id}/password", patch(update_user_password))
        .route("/users/{user_id}", delete(delete_user))
}

async fn require_admin(
    state: &WebAppState,
    jar: &CookieJar,
) -> Result<crate::web::contracts::SessionUser, WebError> {
    let user = session_user_from_jar(state, jar).await?;
    if user.role != UserRole::Admin {
        return Err(WebError::Forbidden("Administrator access is required."));
    }
    Ok(user)
}

async fn get_system_metrics(
    State(state): State<WebAppState>,
    jar: CookieJar,
) -> Result<Json<SystemMetricsResponse>, WebError> {
    let _user = session_user_from_jar(&state, &jar).await?;

    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything()),
    );
    let pid = get_current_pid()
        .map_err(|error| WebError::internal(error.to_string()))?;
    system.refresh_cpu_usage();
    thread::sleep(Duration::from_millis(120));
    system.refresh_cpu_usage();
    system.refresh_memory();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::everything(),
    );

    let process = system.process(pid).ok_or(WebError::internal(
        "Current EmbyStream process was not found.",
    ))?;
    let cpu_core_count = system.cpus().len().max(1) as f64;
    let process_cpu_usage =
        (process.cpu_usage() as f64 / cpu_core_count).clamp(0.0, 100.0);
    let memory_total = system.total_memory();
    let memory_used = process.memory();
    let (disk_used, disk_total) = calculate_tracked_disk_stats(&state.config)?;

    Ok(Json(SystemMetricsResponse {
        cpu_usage_percent: process_cpu_usage,
        cpu_core_count: system.cpus().len() as u32,
        memory_used_bytes: memory_used,
        memory_total_bytes: memory_total,
        memory_usage_percent: percentage(memory_used, memory_total),
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        disk_usage_percent: percentage(disk_used, disk_total),
        uptime_seconds: state.started_at.elapsed().as_secs(),
    }))
}

fn calculate_tracked_disk_stats(
    config: &crate::web::app::WebRuntimeConfig,
) -> Result<(u64, u64), WebError> {
    let mut roots = BTreeSet::new();
    roots.insert(config.executable_path.clone());
    roots.insert(config.data_dir.clone());
    roots.insert(config.runtime_log_dir.clone());
    roots.insert(config.stream_log_dir.clone());

    if let Some(main_config_path) = config.main_config_path.as_ref() {
        roots.insert(main_config_path.clone());
    }

    let used_bytes = roots.iter().try_fold(0_u64, |total, path| {
        tracked_path_size(path).map(|size| total.saturating_add(size))
    })?;

    let current_dir = std::env::current_dir().unwrap_or_default();
    let resolved_roots = roots
        .iter()
        .map(|path| resolve_path(path, &current_dir))
        .collect::<Vec<_>>();
    let disks = Disks::new_with_refreshed_list();
    let mut counted_mounts = BTreeSet::new();
    let mut total_bytes = 0_u64;

    for path in resolved_roots {
        if let Some(disk) = disks
            .list()
            .iter()
            .filter(|disk| path.starts_with(disk.mount_point()))
            .max_by_key(|disk| disk.mount_point().as_os_str().len())
        {
            let mount = disk.mount_point().to_path_buf();
            if counted_mounts.insert(mount) {
                total_bytes = total_bytes.saturating_add(disk.total_space());
            }
        }
    }

    Ok((used_bytes, total_bytes))
}

fn tracked_path_size(path: &FsPath) -> Result<u64, WebError> {
    if !path.exists() {
        return Ok(0);
    }

    let metadata = fs::symlink_metadata(path)?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }

    if metadata.is_dir() {
        return fs::read_dir(path)?.try_fold(0_u64, |total, entry| {
            let entry = entry?;
            let size = tracked_path_size(&entry.path())?;
            Ok::<u64, WebError>(total.saturating_add(size))
        });
    }

    Ok(0)
}

fn resolve_path(path: &FsPath, current_dir: &FsPath) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        current_dir.join(path)
    }
}

async fn list_users(
    State(state): State<WebAppState>,
    jar: CookieJar,
) -> Result<Json<UserListResponse>, WebError> {
    let _admin = require_admin(&state, &jar).await?;
    let items = state.db.list_users().await?;
    Ok(Json(UserListResponse { items }))
}

async fn update_user_role(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUserRoleRequest>,
) -> Result<Json<UserEnvelope>, WebError> {
    let admin = require_admin(&state, &jar).await?;
    forbid_builtin_admin_mutation(&state, &user_id).await?;
    if admin.id == user_id {
        return Err(WebError::Forbidden(
            "You cannot change your own role from this page.",
        ));
    }

    let user = state.db.update_user_role(&user_id, payload.role).await?;
    state
        .db
        .write_audit_log(
            Some(admin.id),
            "update_user_role",
            "user",
            Some(user.id.clone()),
            json!({ "role": user.role }),
        )
        .await?;
    Ok(Json(UserEnvelope { user }))
}

async fn update_user_disabled(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUserDisabledRequest>,
) -> Result<Json<UserEnvelope>, WebError> {
    let admin = require_admin(&state, &jar).await?;
    forbid_builtin_admin_mutation(&state, &user_id).await?;
    if admin.id == user_id {
        return Err(WebError::Forbidden(
            "You cannot disable your own account from this page.",
        ));
    }

    let user = state
        .db
        .update_user_disabled(&user_id, payload.disabled)
        .await?;
    state
        .db
        .write_audit_log(
            Some(admin.id),
            "update_user_disabled",
            "user",
            Some(user.id.clone()),
            json!({ "disabled": user.disabled }),
        )
        .await?;
    Ok(Json(UserEnvelope { user }))
}

async fn update_user_password(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(user_id): Path<String>,
    Json(payload): Json<UpdateUserPasswordRequest>,
) -> Result<Json<UserEnvelope>, WebError> {
    let admin = require_admin(&state, &jar).await?;
    if payload.password.len() < 8 {
        return Err(WebError::invalid_input(
            "password",
            "Password must be at least 8 characters.",
        ));
    }

    let password_hash = hash_password(&payload.password)?;
    let user = state
        .db
        .update_user_password(&user_id, password_hash)
        .await?;
    state
        .db
        .write_audit_log(
            Some(admin.id),
            "update_user_password",
            "user",
            Some(user.id.clone()),
            json!({ "username": user.username }),
        )
        .await?;
    Ok(Json(UserEnvelope { user }))
}

async fn delete_user(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(user_id): Path<String>,
) -> Result<Json<LogoutResponse>, WebError> {
    let admin = require_admin(&state, &jar).await?;
    forbid_builtin_admin_mutation(&state, &user_id).await?;
    if admin.id == user_id {
        return Err(WebError::Forbidden(
            "You cannot delete your own account from this page.",
        ));
    }

    state.db.delete_user(&user_id).await?;
    state
        .db
        .write_audit_log(
            Some(admin.id),
            "delete_user",
            "user",
            Some(user_id),
            json!({}),
        )
        .await?;
    Ok(Json(LogoutResponse { ok: true }))
}

fn percentage(used: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }

    (used as f64 / total as f64) * 100.0
}

async fn forbid_builtin_admin_mutation(
    state: &WebAppState,
    user_id: &str,
) -> Result<(), WebError> {
    let user = state
        .db
        .find_user_by_id(user_id.to_string())
        .await?
        .ok_or(WebError::NotFound("User was not found."))?;

    if user.username == "admin" && user.role == UserRole::Admin {
        return Err(WebError::Forbidden(
            "The built-in administrator cannot be changed from this page.",
        ));
    }

    Ok(())
}
