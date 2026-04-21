use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderMap, Method, header},
    middleware::Next,
    response::Response,
    routing::{get, patch, post},
};
use axum_extra::extract::CookieJar;
use chrono::Utc;
use reqwest::Url;
use serde_json::json;
use std::time::{Duration, Instant};

use super::{
    api::WebAppState,
    contracts::{
        AuthResponse, ChangeOwnPasswordRequest, LoginRequest, LogoutResponse,
        RegisterRequest, SessionUser,
    },
    error::WebError,
};

pub const SESSION_COOKIE_NAME: &str = "embystream_web_session";
const LOGIN_LIMIT_WINDOW: Duration = Duration::from_secs(10 * 60);
const LOGIN_BLOCK_DURATION: Duration = Duration::from_secs(15 * 60);
const MAX_LOGIN_ATTEMPTS: usize = 10;

pub fn routes() -> axum::Router<WebAppState> {
    axum::Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/password", patch(change_own_password))
        .route("/me", get(current_user))
}

pub fn hash_password(password: &str) -> Result<String, WebError> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)?
        .to_string())
}

fn verify_password(
    password: &str,
    password_hash: &str,
) -> Result<bool, WebError> {
    let parsed_hash = PasswordHash::new(password_hash)
        .map_err(|error| WebError::internal(error.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

pub async fn session_user_from_jar(
    state: &WebAppState,
    jar: &CookieJar,
) -> Result<SessionUser, WebError> {
    let session_cookie = jar
        .get(SESSION_COOKIE_NAME)
        .ok_or(WebError::Unauthorized("Session is required."))?;

    let session = state
        .db
        .find_session(session_cookie.value().to_string())
        .await?
        .ok_or(WebError::Unauthorized("Session is not valid."))?;

    Ok(session.user)
}

pub async fn enforce_same_origin(
    request: Request,
    next: Next,
) -> Result<Response, WebError> {
    if !is_unsafe_method(request.method()) {
        return Ok(next.run(request).await);
    }

    validate_same_origin_headers(request.headers())?;
    Ok(next.run(request).await)
}

async fn register(
    State(state): State<WebAppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, WebError> {
    validate_same_origin_headers(&headers)?;
    validate_username(&payload.username)?;
    validate_password(&payload.password)?;

    let email = normalize_optional_string(payload.email);
    let password_hash = hash_password(&payload.password)?;
    let user = state
        .db
        .create_user(
            payload.username.trim().to_string(),
            email,
            password_hash,
            super::contracts::UserRole::User,
        )
        .await?;

    state
        .db
        .write_audit_log(
            Some(user.id.clone()),
            "register",
            "user",
            Some(user.id.clone()),
            json!({ "username": user.username }),
        )
        .await?;

    Ok(Json(AuthResponse { user }))
}

async fn login(
    State(state): State<WebAppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<AuthResponse>), WebError> {
    validate_same_origin_headers(&headers)?;
    if payload.login.trim().is_empty() {
        return Err(WebError::invalid_input("login", "Login is required."));
    }
    validate_password(&payload.password)?;
    let throttle_key = login_throttle_key(&headers);
    enforce_login_rate_limit(&state, &throttle_key)?;

    let Some(user_row) = state
        .db
        .find_user_by_login(payload.login.trim().to_string())
        .await?
    else {
        record_login_failure(&state, &throttle_key);
        return Err(WebError::Unauthorized("Invalid credentials."));
    };

    if user_row.disabled {
        return Err(WebError::Forbidden("This account is disabled."));
    }

    if !verify_password(&payload.password, &user_row.password_hash)? {
        record_login_failure(&state, &throttle_key);
        return Err(WebError::Unauthorized("Invalid credentials."));
    }

    let user = SessionUser {
        id: user_row.id,
        username: user_row.username,
        email: user_row.email,
        role: user_row.role,
    };

    let session_id = state
        .db
        .create_session(
            user.clone(),
            headers
                .get(axum::http::header::USER_AGENT)
                .and_then(|value| value.to_str().ok())
                .map(ToString::to_string),
            None,
        )
        .await?;

    state
        .db
        .write_audit_log(
            Some(user.id.clone()),
            "login",
            "session",
            Some(session_id.clone()),
            json!({ "username": user.username, "at": Utc::now() }),
        )
        .await?;

    clear_login_failures(&state, &throttle_key);
    let cookie = build_session_cookie(&session_id, &headers);
    Ok((jar.add(cookie), Json(AuthResponse { user })))
}

async fn logout(
    State(state): State<WebAppState>,
    jar: CookieJar,
    headers: HeaderMap,
) -> Result<(CookieJar, Json<LogoutResponse>), WebError> {
    validate_same_origin_headers(&headers)?;
    if let Some(cookie) = jar.get(SESSION_COOKIE_NAME) {
        if let Some(session) =
            state.db.find_session(cookie.value().to_string()).await?
        {
            state
                .db
                .write_audit_log(
                    Some(session.user.id.clone()),
                    "logout",
                    "session",
                    Some(session.id.clone()),
                    json!({ "username": session.user.username }),
                )
                .await?;
            state.db.delete_session(session.id).await?;
        }
    }

    let mut remove_cookie =
        axum_extra::extract::cookie::Cookie::build((SESSION_COOKIE_NAME, ""))
            .path("/")
            .http_only(true)
            .same_site(axum_extra::extract::cookie::SameSite::Lax)
            .secure(request_uses_https(&headers))
            .build();
    remove_cookie.make_removal();

    Ok((jar.remove(remove_cookie), Json(LogoutResponse { ok: true })))
}

async fn current_user(
    State(state): State<WebAppState>,
    jar: CookieJar,
) -> Result<Json<AuthResponse>, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    Ok(Json(AuthResponse { user }))
}

async fn change_own_password(
    State(state): State<WebAppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(payload): Json<ChangeOwnPasswordRequest>,
) -> Result<Json<LogoutResponse>, WebError> {
    validate_same_origin_headers(&headers)?;
    let user = session_user_from_jar(&state, &jar).await?;

    if payload.current_password.trim().is_empty() {
        return Err(WebError::invalid_input(
            "current_password",
            "Current password is required.",
        ));
    }
    validate_password(&payload.new_password)?;

    let user_row = state
        .db
        .find_user_row_by_id(user.id.clone())
        .await?
        .ok_or(WebError::Unauthorized("Session is not valid."))?;

    if !verify_password(&payload.current_password, &user_row.password_hash)? {
        return Err(WebError::invalid_input(
            "current_password",
            "Current password is incorrect.",
        ));
    }

    if payload.current_password == payload.new_password {
        return Err(WebError::invalid_input(
            "new_password",
            "New password must be different from the current password.",
        ));
    }

    let password_hash = hash_password(&payload.new_password)?;
    state
        .db
        .update_user_password(&user.id, password_hash)
        .await?;
    state
        .db
        .write_audit_log(
            Some(user.id.clone()),
            "change_own_password",
            "user",
            Some(user.id.clone()),
            json!({ "username": user.username }),
        )
        .await?;

    Ok(Json(LogoutResponse { ok: true }))
}

fn validate_username(username: &str) -> Result<(), WebError> {
    if username.trim().len() < 3 {
        return Err(WebError::invalid_input(
            "username",
            "Username must be at least 3 characters.",
        ));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), WebError> {
    if password.len() < 8 {
        return Err(WebError::invalid_input(
            "password",
            "Password must be at least 8 characters.",
        ));
    }
    Ok(())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn build_session_cookie(
    session_id: &str,
    headers: &HeaderMap,
) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((
        SESSION_COOKIE_NAME,
        session_id.to_string(),
    ))
    .path("/")
    .http_only(true)
    .same_site(axum_extra::extract::cookie::SameSite::Lax)
    .secure(request_uses_https(headers))
    .build()
}

fn is_unsafe_method(method: &Method) -> bool {
    matches!(
        *method,
        Method::POST | Method::PUT | Method::PATCH | Method::DELETE
    )
}

fn validate_same_origin_headers(headers: &HeaderMap) -> Result<(), WebError> {
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };

    if let Some(origin) = headers.get(header::ORIGIN) {
        let origin = origin.to_str().map_err(|_| {
            WebError::Forbidden("Cross-site request was blocked.")
        })?;
        validate_source_url(origin, &host)?;
        return Ok(());
    }

    if let Some(referer) = headers.get(header::REFERER) {
        let referer = referer.to_str().map_err(|_| {
            WebError::Forbidden("Cross-site request was blocked.")
        })?;
        validate_source_url(referer, &host)?;
    }

    Ok(())
}

fn validate_source_url(url: &str, expected_host: &str) -> Result<(), WebError> {
    let parsed = Url::parse(url)
        .map_err(|_| WebError::Forbidden("Cross-site request was blocked."))?;
    let mut authority = parsed
        .host_str()
        .ok_or(WebError::Forbidden("Cross-site request was blocked."))?
        .to_ascii_lowercase();
    if let Some(port) = parsed.port() {
        authority.push(':');
        authority.push_str(&port.to_string());
    }

    if authority != expected_host {
        return Err(WebError::Forbidden("Cross-site request was blocked."));
    }

    Ok(())
}

fn request_uses_https(headers: &HeaderMap) -> bool {
    if headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.split(',').next().unwrap_or("").trim())
        .map(|value| value.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
    {
        return true;
    }

    if headers
        .get("forwarded")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_ascii_lowercase().contains("proto=https"))
        .unwrap_or(false)
    {
        return true;
    }

    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            headers
                .get(header::REFERER)
                .and_then(|value| value.to_str().ok())
        })
        .map(|value| value.starts_with("https://"))
        .unwrap_or(false)
}

fn login_throttle_key(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|value| value.to_str().ok())
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn enforce_login_rate_limit(
    state: &WebAppState,
    throttle_key: &str,
) -> Result<(), WebError> {
    let now = Instant::now();
    if let Some(mut entry) = state.login_attempts.get_mut(throttle_key) {
        entry.failed_attempts.retain(|attempted_at| {
            now.duration_since(*attempted_at) <= LOGIN_LIMIT_WINDOW
        });

        if let Some(blocked_until) = entry.blocked_until {
            if blocked_until > now {
                return Err(WebError::RateLimited(
                    "Too many failed login attempts. Try again later.",
                ));
            }
            entry.blocked_until = None;
        }
    }

    Ok(())
}

fn record_login_failure(state: &WebAppState, throttle_key: &str) {
    let now = Instant::now();
    let mut entry = state
        .login_attempts
        .entry(throttle_key.to_string())
        .or_default();
    entry.failed_attempts.retain(|attempted_at| {
        now.duration_since(*attempted_at) <= LOGIN_LIMIT_WINDOW
    });
    entry.failed_attempts.push(now);
    if entry.failed_attempts.len() >= MAX_LOGIN_ATTEMPTS {
        entry.blocked_until = Some(now + LOGIN_BLOCK_DURATION);
        entry.failed_attempts.clear();
    }
}

fn clear_login_failures(state: &WebAppState, throttle_key: &str) {
    state.login_attempts.remove(throttle_key);
}
