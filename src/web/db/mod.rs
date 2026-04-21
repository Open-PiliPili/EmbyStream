use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use rand::{Rng, distributions::Alphanumeric};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;
use tokio::task;
use uuid::Uuid;

use super::{
    artifacts::RenderedArtifact,
    contracts::{
        ArtifactListResponse, ConfigSetSummary, DraftDocument, DraftRevision,
        DraftStatus, DraftSummary, GenerateDraftResponse, LogEntry,
        LogListResponse, LoginBackgroundResponse, SessionUser,
        UserAdminSummary, UserRole, WizardPayload, WizardStreamMode,
    },
    error::WebError,
};

const DB_FILE_NAME: &str = "web-config-studio.sqlite3";
const SESSION_TTL_HOURS: i64 = 24 * 14;

#[derive(Debug, Clone)]
pub struct BootstrapAdmin {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct Database {
    data_dir: Arc<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct UserRow {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
    pub role: UserRole,
    pub disabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub user: SessionUser,
}

#[derive(Debug, Clone)]
pub struct LogsQuery {
    pub source: Option<String>,
    pub limit: usize,
}

#[derive(Debug)]
pub struct PersistGeneratedConfigInput {
    pub user_id: String,
    pub draft_id: String,
    pub draft_name: String,
    pub payload: WizardPayload,
    pub stream_mode: WizardStreamMode,
    pub config_toml: String,
    pub artifacts: Vec<RenderedArtifact>,
}

impl Default for LogsQuery {
    fn default() -> Self {
        Self {
            source: None,
            limit: 50,
        }
    }
}

impl Database {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            data_dir: Arc::new(data_dir),
        }
    }

    pub fn data_dir(&self) -> &Path {
        self.data_dir.as_ref().as_path()
    }

    pub fn db_path(&self) -> PathBuf {
        self.data_dir().join(DB_FILE_NAME)
    }

    pub async fn initialize(&self) -> Result<Option<BootstrapAdmin>, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || db.initialize_blocking()).await?
    }

    pub async fn create_user(
        &self,
        username: String,
        email: Option<String>,
        password_hash: String,
        role: UserRole,
    ) -> Result<SessionUser, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            let role_value = role.as_db_value();
            let result = conn.execute(
                "INSERT INTO users (
                    id, username, email, password_hash, role, disabled, created_at, updated_at, last_login_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?7, NULL)",
                params![
                    id,
                    username,
                    email,
                    password_hash,
                    role_value,
                    now,
                    now
                ],
            );

            match result {
                Ok(_) => Ok(SessionUser {
                    id,
                    username,
                    email,
                    role,
                }),
                Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                    if message.contains("users.username") =>
                {
                    Err(WebError::Conflict {
                        message: "Username already exists.",
                        field: Some("username"),
                    })
                }
                Err(rusqlite::Error::SqliteFailure(_, Some(message)))
                    if message.contains("users.email") =>
                {
                    Err(WebError::Conflict {
                        message: "Email already exists.",
                        field: Some("email"),
                    })
                }
                Err(error) => Err(WebError::from(error)),
            }
        })
        .await?
    }

    pub async fn find_user_by_login(
        &self,
        login: String,
    ) -> Result<Option<UserRow>, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.query_row(
                "SELECT id, username, email, password_hash, role
                 , disabled, created_at, updated_at
                 FROM users
                 WHERE username = ?1 OR email = ?1
                 LIMIT 1",
                params![login],
                map_user_row,
            )
            .optional()
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn find_user_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<SessionUser>, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.query_row(
                "SELECT id, username, email, role FROM users WHERE id = ?1 LIMIT 1",
                params![user_id],
                |row| {
                    Ok(SessionUser {
                        id: row.get(0)?,
                        username: row.get(1)?,
                        email: row.get(2)?,
                        role: map_user_role_value(row.get::<_, String>(3)?, 3)?,
                    })
                },
            )
            .optional()
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn find_user_row_by_id(
        &self,
        user_id: String,
    ) -> Result<Option<UserRow>, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.query_row(
                "SELECT id, username, email, password_hash, role, disabled, created_at, updated_at
                 FROM users WHERE id = ?1 LIMIT 1",
                params![user_id],
                map_user_row,
            )
            .optional()
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn create_session(
        &self,
        user: SessionUser,
        user_agent: Option<String>,
        ip_address: Option<String>,
    ) -> Result<String, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let session_id = Uuid::new_v4().to_string();
            let now = Utc::now();
            let expires_at =
                (now + chrono::Duration::hours(SESSION_TTL_HOURS)).to_rfc3339();

            conn.execute(
                "INSERT INTO sessions (
                    id, user_id, expires_at, created_at, last_seen_at, user_agent, ip_address
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    user.id,
                    expires_at,
                    now.to_rfc3339(),
                    now.to_rfc3339(),
                    user_agent,
                    ip_address
                ],
            )?;
            Ok(session_id)
        })
        .await?
    }

    pub async fn find_session(
        &self,
        session_id: String,
    ) -> Result<Option<SessionRow>, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let session = conn
                .query_row(
                    "SELECT
                        s.id,
                        s.expires_at,
                        u.id,
                        u.username,
                        u.email,
                        u.role,
                        u.disabled
                     FROM sessions s
                     JOIN users u ON u.id = s.user_id
                     WHERE s.id = ?1
                     LIMIT 1",
                    params![session_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            SessionUser {
                                id: row.get(2)?,
                                username: row.get(3)?,
                                email: row.get(4)?,
                                role: map_user_role_value(
                                    row.get::<_, String>(5)?,
                                    5,
                                )?,
                            },
                            row.get::<_, i64>(6)? != 0,
                        ))
                    },
                )
                .optional()?;

            let Some((session_id, expires_at, user, disabled)) = session else {
                return Ok(None);
            };

            let expires_at = DateTime::parse_from_rfc3339(&expires_at)
                .map_err(|error| WebError::internal(error.to_string()))?
                .with_timezone(&Utc);

            if expires_at <= Utc::now() {
                conn.execute(
                    "DELETE FROM sessions WHERE id = ?1",
                    params![session_id],
                )?;
                return Ok(None);
            }

            if disabled {
                conn.execute(
                    "DELETE FROM sessions WHERE id = ?1",
                    params![session_id],
                )?;
                return Ok(None);
            }

            conn.execute(
                "UPDATE sessions SET last_seen_at = ?2 WHERE id = ?1",
                params![session_id, Utc::now().to_rfc3339()],
            )?;

            Ok(Some(SessionRow {
                id: session_id,
                user,
            }))
        })
        .await?
    }

    pub async fn list_users(&self) -> Result<Vec<UserAdminSummary>, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let mut stmt = conn.prepare(
                "SELECT id, username, email, password_hash, role, disabled, created_at, updated_at
                 FROM users
                 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map([], map_user_row)?;

            let mut items = Vec::new();
            for row in rows {
                items.push(user_admin_summary_from_row(row?)?);
            }

            Ok(items)
        })
        .await?
    }

    pub async fn update_user_role(
        &self,
        user_id: &str,
        role: UserRole,
    ) -> Result<UserAdminSummary, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "UPDATE users SET role = ?1, updated_at = ?2 WHERE id = ?3",
                params![role.as_db_value(), Utc::now().to_rfc3339(), user_id],
            )?;
            let row = conn
                .query_row(
                    "SELECT id, username, email, password_hash, role, disabled, created_at, updated_at
                     FROM users WHERE id = ?1 LIMIT 1",
                    params![user_id],
                    map_user_row,
                )
                .optional()?
                .ok_or(WebError::NotFound("User was not found."))?;
            user_admin_summary_from_row(row)
        })
        .await?
    }

    pub async fn update_user_disabled(
        &self,
        user_id: &str,
        disabled: bool,
    ) -> Result<UserAdminSummary, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "UPDATE users SET disabled = ?1, updated_at = ?2 WHERE id = ?3",
                params![if disabled { 1 } else { 0 }, Utc::now().to_rfc3339(), user_id],
            )?;
            if disabled {
                conn.execute(
                    "DELETE FROM sessions WHERE user_id = ?1",
                    params![user_id.clone()],
                )?;
            }
            let row = conn
                .query_row(
                    "SELECT id, username, email, password_hash, role, disabled, created_at, updated_at
                     FROM users WHERE id = ?1 LIMIT 1",
                    params![user_id],
                    map_user_row,
                )
                .optional()?
                .ok_or(WebError::NotFound("User was not found."))?;
            user_admin_summary_from_row(row)
        })
        .await?
    }

    pub async fn update_user_password(
        &self,
        user_id: &str,
        password_hash: String,
    ) -> Result<UserAdminSummary, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
                params![password_hash, Utc::now().to_rfc3339(), user_id],
            )?;
            conn.execute(
                "DELETE FROM sessions WHERE user_id = ?1",
                params![user_id.clone()],
            )?;
            let row = conn
                .query_row(
                    "SELECT id, username, email, password_hash, role, disabled, created_at, updated_at
                     FROM users WHERE id = ?1 LIMIT 1",
                    params![user_id],
                    map_user_row,
                )
                .optional()?
                .ok_or(WebError::NotFound("User was not found."))?;
            user_admin_summary_from_row(row)
        })
        .await?
    }

    pub async fn delete_user(&self, user_id: &str) -> Result<(), WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let deleted = conn
                .execute("DELETE FROM users WHERE id = ?1", params![user_id])?;
            if deleted == 0 {
                return Err(WebError::NotFound("User was not found."));
            }
            Ok(())
        })
        .await?
    }

    pub async fn delete_session(
        &self,
        session_id: String,
    ) -> Result<(), WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "DELETE FROM sessions WHERE id = ?1",
                params![session_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn write_audit_log(
        &self,
        user_id: Option<String>,
        action: &str,
        target_type: &str,
        target_id: Option<String>,
        detail_json: serde_json::Value,
    ) -> Result<(), WebError> {
        let db = self.clone();
        let action = action.to_string();
        let target_type = target_type.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "INSERT INTO audit_logs (
                    id, user_id, action, target_type, target_id, detail_json, created_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    Uuid::new_v4().to_string(),
                    user_id,
                    action,
                    target_type,
                    target_id,
                    detail_json.to_string(),
                    Utc::now().to_rfc3339()
                ],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn create_draft(
        &self,
        user_id: &str,
        name: String,
        stream_mode: WizardStreamMode,
        payload: WizardPayload,
    ) -> Result<DraftSummary, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let draft_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO drafts (
                    id, user_id, name, status, stream_mode, wizard_version, payload_json,
                    generated_config_toml, created_at, updated_at, last_opened_at
                 ) VALUES (?1, ?2, ?3, 'draft', ?4, 1, ?5, NULL, ?6, ?6, ?6)",
                params![
                    draft_id,
                    user_id,
                    name,
                    stream_mode.as_db_value(),
                    serde_json::to_string(&payload)
                        .map_err(|error| WebError::internal(error.to_string()))?,
                    now
                ],
            )?;

            Ok(DraftSummary {
                id: draft_id,
                name,
                status: DraftStatus::Draft,
                stream_mode,
                updated_at: Utc::now(),
            })
        })
        .await?
    }

    pub async fn list_drafts(
        &self,
        user_id: &str,
    ) -> Result<Vec<DraftSummary>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let mut stmt = conn.prepare(
                "SELECT id, name, status, stream_mode, updated_at
                 FROM drafts
                 WHERE user_id = ?1
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map(params![user_id], |row| {
                Ok(DraftSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    status: map_draft_status_value(
                        row.get::<_, String>(2)?,
                        2,
                    )?,
                    stream_mode: map_stream_mode_value(
                        row.get::<_, String>(3)?,
                        3,
                    )?,
                    updated_at: parse_rfc3339_to_utc(
                        row.get::<_, String>(4)?,
                        4,
                    )?,
                })
            })?;

            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
        .await?
    }

    pub async fn get_draft(
        &self,
        user_id: &str,
        draft_id: &str,
    ) -> Result<Option<DraftDocument>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let draft_id = draft_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "UPDATE drafts SET last_opened_at = ?3
                 WHERE id = ?1 AND user_id = ?2",
                params![draft_id, user_id, Utc::now().to_rfc3339()],
            )?;

            conn.query_row(
                "SELECT id, name, status, stream_mode, payload_json, updated_at
                 FROM drafts
                 WHERE id = ?1 AND user_id = ?2
                 LIMIT 1",
                params![draft_id, user_id],
                |row| {
                    Ok(DraftDocument {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        status: map_draft_status_value(
                            row.get::<_, String>(2)?,
                            2,
                        )?,
                        stream_mode: map_stream_mode_value(
                            row.get::<_, String>(3)?,
                            3,
                        )?,
                        payload: serde_json::from_str::<WizardPayload>(
                            &row.get::<_, String>(4)?,
                        )
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?,
                        updated_at: parse_rfc3339_to_utc(
                            row.get::<_, String>(5)?,
                            5,
                        )?,
                    })
                },
            )
            .optional()
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn save_draft(
        &self,
        user_id: &str,
        draft_id: &str,
        name: String,
        payload: WizardPayload,
        _client_revision: u64,
    ) -> Result<DraftRevision, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let draft_id = draft_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let now = Utc::now().to_rfc3339();
            let updated = conn.execute(
                "UPDATE drafts
                 SET name = ?3, payload_json = ?4, stream_mode = ?5,
                     updated_at = ?6, wizard_version = wizard_version + 1
                 WHERE id = ?1 AND user_id = ?2",
                params![
                    draft_id,
                    user_id,
                    name,
                    serde_json::to_string(&payload).map_err(|error| {
                        WebError::internal(error.to_string())
                    })?,
                    payload.stream_mode.as_db_value(),
                    now
                ],
            )?;

            if updated == 0 {
                return Err(WebError::NotFound("Draft was not found."));
            }

            conn.query_row(
                "SELECT updated_at, wizard_version
                 FROM drafts
                 WHERE id = ?1 AND user_id = ?2
                 LIMIT 1",
                params![draft_id, user_id],
                |row| {
                    Ok(DraftRevision {
                        id: draft_id.clone(),
                        updated_at: parse_rfc3339_to_utc(
                            row.get::<_, String>(0)?,
                            0,
                        )?,
                        server_revision: row.get::<_, u64>(1)?,
                    })
                },
            )
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn delete_draft(
        &self,
        user_id: &str,
        draft_id: &str,
    ) -> Result<(), WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let draft_id = draft_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "DELETE FROM drafts WHERE id = ?1 AND user_id = ?2",
                params![draft_id, user_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn update_draft_metadata(
        &self,
        user_id: &str,
        draft_id: &str,
        name: String,
    ) -> Result<Option<DraftSummary>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let draft_id = draft_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let now = Utc::now().to_rfc3339();
            let updated = conn.execute(
                "UPDATE drafts
                 SET name = ?3, updated_at = ?4
                 WHERE id = ?1 AND user_id = ?2",
                params![draft_id, user_id, name, now],
            )?;

            if updated == 0 {
                return Ok(None);
            }

            conn.query_row(
                "SELECT id, name, status, stream_mode, updated_at
                 FROM drafts
                 WHERE id = ?1 AND user_id = ?2
                 LIMIT 1",
                params![draft_id, user_id],
                |row| {
                    Ok(DraftSummary {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        status: map_draft_status_value(
                            row.get::<_, String>(2)?,
                            2,
                        )?,
                        stream_mode: map_stream_mode_value(
                            row.get::<_, String>(3)?,
                            3,
                        )?,
                        updated_at: parse_rfc3339_to_utc(
                            row.get::<_, String>(4)?,
                            4,
                        )?,
                    })
                },
            )
            .map(Some)
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn persist_generated_config(
        &self,
        input: PersistGeneratedConfigInput,
    ) -> Result<GenerateDraftResponse, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let mut conn = db.open_connection()?;
            let tx = conn.transaction()?;
            let config_set_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            let payload_json = serde_json::to_string(&input.payload)
                .map_err(|error| WebError::internal(error.to_string()))?;

            tx.execute(
                "INSERT INTO config_sets (
                    id, user_id, source_draft_id, name, stream_mode, payload_json,
                    created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
                params![
                    config_set_id,
                    input.user_id,
                    input.draft_id,
                    input.draft_name,
                    input.stream_mode.as_db_value(),
                    payload_json,
                    now
                ],
            )?;

            for artifact in &input.artifacts {
                tx.execute(
                    "INSERT INTO artifacts (
                        id, config_set_id, artifact_type, file_name, content, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        Uuid::new_v4().to_string(),
                        config_set_id,
                        artifact.artifact_type.as_db_value(),
                        artifact.file_name,
                        artifact.content,
                        now
                    ],
                )?;
            }

            tx.execute(
                "UPDATE drafts
                 SET status = 'generated',
                     stream_mode = ?3,
                     payload_json = ?4,
                     generated_config_toml = ?5,
                     updated_at = ?6
                 WHERE id = ?1 AND user_id = ?2",
                params![
                    input.draft_id,
                    input.user_id,
                    input.stream_mode.as_db_value(),
                    serde_json::to_string(&input.payload)
                        .map_err(|error| WebError::internal(error.to_string()))?,
                    input.config_toml,
                    now
                ],
            )?;

            tx.commit()?;

            Ok(GenerateDraftResponse {
                config_set: ConfigSetSummary {
                    id: config_set_id,
                    name: input.draft_name,
                    stream_mode: input.stream_mode,
                    created_at: parse_rfc3339_to_utc(now.clone(), 0)?,
                    updated_at: parse_rfc3339_to_utc(now, 0)?,
                },
                artifacts: input
                    .artifacts
                    .into_iter()
                    .map(|artifact| artifact.summary())
                    .collect(),
            })
        })
        .await?
    }

    pub async fn list_config_sets(
        &self,
        user_id: &str,
    ) -> Result<Vec<ConfigSetSummary>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let mut stmt = conn.prepare(
                "SELECT id, name, stream_mode, created_at, updated_at
                 FROM config_sets
                 WHERE user_id = ?1
                 ORDER BY updated_at DESC",
            )?;
            let rows = stmt.query_map(params![user_id], |row| {
                Ok(ConfigSetSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    stream_mode: map_stream_mode_value(
                        row.get::<_, String>(2)?,
                        2,
                    )?,
                    created_at: parse_rfc3339_to_utc(
                        row.get::<_, String>(3)?,
                        3,
                    )?,
                    updated_at: parse_rfc3339_to_utc(
                        row.get::<_, String>(4)?,
                        4,
                    )?,
                })
            })?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(items)
        })
        .await?
    }

    pub async fn list_config_set_artifacts(
        &self,
        user_id: &str,
        config_set_id: &str,
    ) -> Result<Option<ArtifactListResponse>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let config_set_id = config_set_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let exists = conn
                .query_row(
                    "SELECT id FROM config_sets WHERE id = ?1 AND user_id = ?2 LIMIT 1",
                    params![config_set_id, user_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if exists.is_none() {
                return Ok(None);
            }

            let mut stmt = conn.prepare(
                "SELECT artifact_type, file_name, content
                 FROM artifacts
                 WHERE config_set_id = ?1
                 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![config_set_id], |row| {
                let artifact_type = map_artifact_type_value(
                    row.get::<_, String>(0)?,
                    0,
                )?;
                Ok(artifact_type.document(
                    row.get::<_, String>(1)?,
                    artifact_language(artifact_type).to_string(),
                    row.get::<_, String>(2)?,
                ))
            })?;
            let mut items = Vec::new();
            for row in rows {
                items.push(row?);
            }
            Ok(Some(ArtifactListResponse { items }))
        })
        .await?
    }

    pub async fn duplicate_config_set(
        &self,
        user_id: &str,
        config_set_id: &str,
    ) -> Result<Option<DraftSummary>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let config_set_id = config_set_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let source = conn
                .query_row(
                    "SELECT name, stream_mode, payload_json
                     FROM config_sets
                     WHERE id = ?1 AND user_id = ?2
                     LIMIT 1",
                    params![config_set_id, user_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            map_stream_mode_value(
                                row.get::<_, String>(1)?,
                                1,
                            )?,
                            serde_json::from_str::<WizardPayload>(
                                &row.get::<_, String>(2)?,
                            )
                            .map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    2,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })?,
                        ))
                    },
                )
                .optional()?;

            let Some((name, stream_mode, payload)) = source else {
                return Ok(None);
            };

            let draft_id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            let draft_name = format!("{name} copy");
            conn.execute(
                "INSERT INTO drafts (
                    id, user_id, name, status, stream_mode, wizard_version, payload_json,
                    generated_config_toml, created_at, updated_at, last_opened_at
                 ) VALUES (?1, ?2, ?3, 'draft', ?4, 1, ?5, NULL, ?6, ?6, ?6)",
                params![
                    draft_id,
                    user_id,
                    draft_name,
                    stream_mode.as_db_value(),
                    serde_json::to_string(&payload)
                        .map_err(|error| WebError::internal(error.to_string()))?,
                    now
                ],
            )?;

            Ok(Some(DraftSummary {
                id: draft_id,
                name: draft_name,
                status: DraftStatus::Draft,
                stream_mode,
                updated_at: parse_rfc3339_to_utc(now, 0)?,
            }))
        })
        .await?
    }

    pub async fn delete_config_set(
        &self,
        user_id: &str,
        config_set_id: &str,
    ) -> Result<(), WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let config_set_id = config_set_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.execute(
                "DELETE FROM config_sets WHERE id = ?1 AND user_id = ?2",
                params![config_set_id, user_id],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn update_config_set_metadata(
        &self,
        user_id: &str,
        config_set_id: &str,
        name: String,
    ) -> Result<Option<ConfigSetSummary>, WebError> {
        let db = self.clone();
        let user_id = user_id.to_string();
        let config_set_id = config_set_id.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let now = Utc::now().to_rfc3339();
            let updated = conn.execute(
                "UPDATE config_sets
                 SET name = ?3, updated_at = ?4
                 WHERE id = ?1 AND user_id = ?2",
                params![config_set_id, user_id, name, now],
            )?;

            if updated == 0 {
                return Ok(None);
            }

            conn.query_row(
                "SELECT id, name, stream_mode, created_at, updated_at
                 FROM config_sets
                 WHERE id = ?1 AND user_id = ?2
                 LIMIT 1",
                params![config_set_id, user_id],
                |row| {
                    Ok(ConfigSetSummary {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        stream_mode: map_stream_mode_value(
                            row.get::<_, String>(2)?,
                            2,
                        )?,
                        created_at: parse_rfc3339_to_utc(
                            row.get::<_, String>(3)?,
                            3,
                        )?,
                        updated_at: parse_rfc3339_to_utc(
                            row.get::<_, String>(4)?,
                            4,
                        )?,
                    })
                },
            )
            .map(Some)
            .map_err(WebError::from)
        })
        .await?
    }

    pub async fn reset_admin_password(
        &self,
        username: &str,
    ) -> Result<String, WebError> {
        let password = generate_password();
        let password_hash = crate::web::auth::hash_password(&password)?;
        let db = self.clone();
        let username = username.to_string();

        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let updated = conn.execute(
                "UPDATE users
                 SET password_hash = ?1, updated_at = ?2
                 WHERE username = ?3 AND role = 'admin'",
                params![password_hash, Utc::now().to_rfc3339(), username],
            )?;

            if updated == 0 {
                return Err(WebError::NotFound(
                    "Administrator user was not found.",
                ));
            }

            Ok(password)
        })
        .await?
    }

    pub async fn load_background_cache(
        &self,
        cache_key: &str,
    ) -> Result<Option<LoginBackgroundResponse>, WebError> {
        let db = self.clone();
        let cache_key = cache_key.to_string();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            conn.query_row(
                "SELECT payload_json
                 FROM background_cache
                 WHERE cache_key = ?1
                 ORDER BY fetched_at DESC
                 LIMIT 1",
                params![cache_key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(WebError::from)
            .and_then(|value| {
                value.map_or(Ok(None), |payload| {
                    serde_json::from_str::<LoginBackgroundResponse>(&payload)
                        .map(Some)
                        .map_err(|error| WebError::internal(error.to_string()))
                })
            })
        })
        .await?
    }

    pub async fn save_background_cache(
        &self,
        cache_key: &str,
        response: &LoginBackgroundResponse,
    ) -> Result<(), WebError> {
        let db = self.clone();
        let cache_key = cache_key.to_string();
        let response = response.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let payload_json = serde_json::to_string(&response)
                .map_err(|error| WebError::internal(error.to_string()))?;
            conn.execute(
                "DELETE FROM background_cache WHERE cache_key = ?1",
                params![cache_key],
            )?;
            conn.execute(
                "INSERT INTO background_cache (
                    provider, cache_key, payload_json, fetched_at, expires_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    match response.provider {
                        super::contracts::BackgroundProvider::Tmdb => "tmdb",
                        super::contracts::BackgroundProvider::Bing => "bing",
                        super::contracts::BackgroundProvider::StaticFallback => {
                            "static_fallback"
                        }
                    },
                    cache_key,
                    payload_json,
                    response.fetched_at.to_rfc3339(),
                    response.expires_at.to_rfc3339()
                ],
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn list_logs(
        &self,
        query: LogsQuery,
    ) -> Result<LogListResponse, WebError> {
        let db = self.clone();
        task::spawn_blocking(move || {
            let conn = db.open_connection()?;
            let mut items = Vec::new();

            if query
                .source
                .as_deref()
                .map(|value| value == "runtime")
                .unwrap_or(false)
            {
                return Ok(LogListResponse {
                    items,
                    next_cursor: None,
                });
            }

            let mut stmt = conn.prepare(
                "SELECT created_at, action, target_type, detail_json
                 FROM audit_logs
                 ORDER BY created_at DESC
                 LIMIT ?1",
            )?;

            let rows = stmt.query_map(params![query.limit as i64], |row| {
                let detail_json: String = row.get(3)?;
                let detail_json =
                    serde_json::from_str::<serde_json::Value>(&detail_json)
                        .unwrap_or_else(|_| json!({}));
                let message =
                    if detail_json.is_null() || detail_json == json!({}) {
                        format!(
                            "{} {}",
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?
                        )
                    } else {
                        format!(
                            "{} {} {}",
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            detail_json
                        )
                    };
                Ok(LogEntry {
                    timestamp: DateTime::parse_from_rfc3339(
                        &row.get::<_, String>(0)?,
                    )
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?
                    .with_timezone(&Utc),
                    level: "INFO".to_string(),
                    source: "audit".to_string(),
                    message,
                })
            })?;

            for row in rows {
                items.push(row?);
            }

            Ok(LogListResponse {
                items,
                next_cursor: None,
            })
        })
        .await?
    }

    fn initialize_blocking(&self) -> Result<Option<BootstrapAdmin>, WebError> {
        std::fs::create_dir_all(self.data_dir())?;

        for subdir in ["artifacts", "backgrounds"] {
            std::fs::create_dir_all(self.data_dir().join(subdir))?;
        }

        let conn = self.open_connection()?;
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                username TEXT NOT NULL UNIQUE,
                email TEXT UNIQUE,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('admin', 'user')),
                disabled INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_login_at TEXT
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_seen_at TEXT NOT NULL,
                user_agent TEXT,
                ip_address TEXT,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS drafts (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                name TEXT NOT NULL,
                status TEXT NOT NULL CHECK (status IN ('draft', 'generated', 'archived')),
                stream_mode TEXT NOT NULL CHECK (stream_mode IN ('frontend', 'backend', 'dual')),
                wizard_version INTEGER NOT NULL DEFAULT 1,
                payload_json TEXT NOT NULL DEFAULT '{}',
                generated_config_toml TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                last_opened_at TEXT,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS config_sets (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                source_draft_id TEXT,
                name TEXT NOT NULL,
                stream_mode TEXT NOT NULL CHECK (stream_mode IN ('frontend', 'backend', 'dual')),
                payload_json TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY(source_draft_id) REFERENCES drafts(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS artifacts (
                id TEXT PRIMARY KEY,
                config_set_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL CHECK (artifact_type IN (
                    'config_toml',
                    'nginx_conf',
                    'docker_compose',
                    'systemd_service',
                    'pm2_config'
                )),
                file_name TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(config_set_id) REFERENCES config_sets(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS background_cache (
                provider TEXT NOT NULL,
                cache_key TEXT NOT NULL,
                payload_json TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                PRIMARY KEY(provider, cache_key)
            );

            CREATE TABLE IF NOT EXISTS audit_logs (
                id TEXT PRIMARY KEY,
                user_id TEXT,
                action TEXT NOT NULL,
                target_type TEXT NOT NULL,
                target_id TEXT,
                detail_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE SET NULL
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_user_id ON sessions(user_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);
            CREATE INDEX IF NOT EXISTS idx_drafts_user_id ON drafts(user_id);
            CREATE INDEX IF NOT EXISTS idx_config_sets_user_id ON config_sets(user_id);
            CREATE INDEX IF NOT EXISTS idx_artifacts_config_set_id ON artifacts(config_set_id);
            CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
            "#,
        )?;

        let _ = conn.execute(
            "ALTER TABLE users ADD COLUMN disabled INTEGER NOT NULL DEFAULT 0",
            [],
        );

        let admin_exists: Option<String> = conn
            .query_row(
                "SELECT username FROM users WHERE role = 'admin' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;

        if admin_exists.is_some() {
            return Ok(None);
        }

        let password = generate_password();
        let password_hash = crate::web::auth::hash_password(&password)?;
        let now = Utc::now().to_rfc3339();
        let user_id = Uuid::new_v4().to_string();

        conn.execute(
            "INSERT INTO users (
                id, username, email, password_hash, role, disabled, created_at, updated_at, last_login_at
             ) VALUES (?1, 'admin', NULL, ?2, 'admin', 0, ?3, ?3, NULL)",
            params![user_id, password_hash, now],
        )?;

        conn.execute(
            "INSERT INTO audit_logs (
                id, user_id, action, target_type, target_id, detail_json, created_at
             ) VALUES (?1, ?2, 'bootstrap_admin', 'user', ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                user_id,
                json!({"username": "admin"}).to_string(),
                now
            ],
        )?;

        Ok(Some(BootstrapAdmin {
            username: "admin".to_string(),
            password,
        }))
    }

    fn open_connection(&self) -> Result<Connection, WebError> {
        let connection = Connection::open(self.db_path())?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Ok(connection)
    }
}

fn generate_password() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .filter(|character| character.is_ascii_alphanumeric())
        .take(20)
        .collect()
}

fn map_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRow> {
    let role = map_user_role_value(row.get::<_, String>(4)?, 4)?;

    Ok(UserRow {
        id: row.get(0)?,
        username: row.get(1)?,
        email: row.get(2)?,
        password_hash: row.get(3)?,
        role,
        disabled: row.get::<_, i64>(5)? != 0,
        created_at: parse_rfc3339_to_utc(row.get::<_, String>(6)?, 6)?,
        updated_at: parse_rfc3339_to_utc(row.get::<_, String>(7)?, 7)?,
    })
}

fn user_admin_summary_from_row(
    row: UserRow,
) -> Result<UserAdminSummary, WebError> {
    Ok(UserAdminSummary {
        id: row.id,
        username: row.username,
        email: row.email,
        role: row.role,
        disabled: row.disabled,
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}

fn map_user_role_value(
    value: String,
    index: usize,
) -> rusqlite::Result<UserRole> {
    UserRole::from_db_value(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

impl UserRole {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::User => "user",
        }
    }

    pub fn from_db_value(value: &str) -> Result<Self, WebError> {
        match value {
            "admin" => Ok(Self::Admin),
            "user" => Ok(Self::User),
            _ => Err(WebError::internal(format!("Unknown role '{value}'"))),
        }
    }
}

impl WizardStreamMode {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Frontend => "frontend",
            Self::Backend => "backend",
            Self::Dual => "dual",
        }
    }
}

impl DraftStatus {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Generated => "generated",
            Self::Archived => "archived",
        }
    }
}

impl super::contracts::ArtifactType {
    pub fn as_db_value(self) -> &'static str {
        match self {
            Self::ConfigToml => "config_toml",
            Self::NginxConf => "nginx_conf",
            Self::DockerCompose => "docker_compose",
            Self::SystemdService => "systemd_service",
            Self::Pm2Config => "pm2_config",
        }
    }

    pub fn document(
        self,
        file_name: String,
        language: String,
        content: String,
    ) -> super::contracts::ArtifactDocument {
        super::contracts::ArtifactDocument {
            artifact_type: self,
            file_name,
            language,
            content,
        }
    }
}

fn artifact_language(
    artifact_type: super::contracts::ArtifactType,
) -> &'static str {
    match artifact_type {
        super::contracts::ArtifactType::ConfigToml => "toml",
        super::contracts::ArtifactType::NginxConf => "nginx",
        super::contracts::ArtifactType::DockerCompose => "yaml",
        super::contracts::ArtifactType::SystemdService => "ini",
        super::contracts::ArtifactType::Pm2Config => "javascript",
    }
}

fn map_stream_mode_value(
    value: String,
    index: usize,
) -> rusqlite::Result<WizardStreamMode> {
    match value.as_str() {
        "frontend" => Ok(WizardStreamMode::Frontend),
        "backend" => Ok(WizardStreamMode::Backend),
        "dual" => Ok(WizardStreamMode::Dual),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(WebError::internal(format!(
                "Unknown stream mode '{value}'"
            ))),
        )),
    }
}

fn map_draft_status_value(
    value: String,
    index: usize,
) -> rusqlite::Result<DraftStatus> {
    match value.as_str() {
        "draft" => Ok(DraftStatus::Draft),
        "generated" => Ok(DraftStatus::Generated),
        "archived" => Ok(DraftStatus::Archived),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(WebError::internal(format!(
                "Unknown draft status '{value}'"
            ))),
        )),
    }
}

fn map_artifact_type_value(
    value: String,
    index: usize,
) -> rusqlite::Result<super::contracts::ArtifactType> {
    match value.as_str() {
        "config_toml" => Ok(super::contracts::ArtifactType::ConfigToml),
        "nginx_conf" => Ok(super::contracts::ArtifactType::NginxConf),
        "docker_compose" => Ok(super::contracts::ArtifactType::DockerCompose),
        "systemd_service" => Ok(super::contracts::ArtifactType::SystemdService),
        "pm2_config" => Ok(super::contracts::ArtifactType::Pm2Config),
        _ => Err(rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(WebError::internal(format!(
                "Unknown artifact type '{value}'"
            ))),
        )),
    }
}

fn parse_rfc3339_to_utc(
    value: String,
    index: usize,
) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}
