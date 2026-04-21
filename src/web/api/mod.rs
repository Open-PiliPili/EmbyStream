use std::{sync::Arc, time::Instant};

use axum::http::{HeaderValue, header};
use axum::middleware;
use axum::{
    Router, extract::Request, middleware::Next, response::Response,
    routing::get,
};
use dashmap::DashMap;
use reqwest::Client;

use super::{
    admin,
    app::WebRuntimeConfig,
    artifacts,
    assets::{FRONTEND_DIST_DIR, has_embedded_assets},
    auth, backgrounds,
    db::Database,
    drafts, logs,
};

#[derive(Debug, Clone)]
pub struct WebAppState {
    pub db: Database,
    pub config: WebRuntimeConfig,
    pub http_client: Client,
    pub started_at: Arc<Instant>,
    pub login_attempts: Arc<DashMap<String, LoginThrottleState>>,
}

#[derive(Debug, Clone)]
pub struct LoginThrottleState {
    pub failed_attempts: Vec<Instant>,
    pub blocked_until: Option<Instant>,
}

impl LoginThrottleState {
    pub fn new() -> Self {
        Self {
            failed_attempts: Vec::new(),
            blocked_until: None,
        }
    }
}

impl Default for LoginThrottleState {
    fn default() -> Self {
        Self::new()
    }
}

impl WebAppState {
    pub fn new(db: Database, config: WebRuntimeConfig) -> Self {
        Self {
            db,
            config,
            http_client: Client::new(),
            started_at: Arc::new(Instant::now()),
            login_attempts: Arc::new(DashMap::new()),
        }
    }
}

pub fn build_router(state: WebAppState) -> Router {
    let api_router = Router::new()
        .nest("/api/auth", auth::routes())
        .nest("/api/drafts", drafts::routes())
        .route("/api/config-sets", get(drafts::list_config_sets))
        .nest("/api/config-sets", artifacts::routes())
        .nest("/api/admin", admin::routes())
        .nest("/api/logs", logs::routes())
        .nest("/api/backgrounds", backgrounds::routes())
        .layer(middleware::from_fn(apply_security_headers))
        .layer(middleware::from_fn(auth::enforce_same_origin));

    let dist_dir = std::path::Path::new(FRONTEND_DIST_DIR);
    let index_html = dist_dir.join("index.html");

    let router = if has_embedded_assets() || index_html.exists() {
        api_router.fallback(get(super::assets::serve_frontend))
    } else {
        api_router
    };

    router.with_state(state)
}

async fn apply_security_headers(request: Request, next: Next) -> Response {
    let is_api = request.uri().path().starts_with("/api/");
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );

    if is_api {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        );
    }

    let is_html = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.starts_with("text/html"))
        .unwrap_or(false);

    if is_html {
        headers.insert(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'; script-src 'self' 'unsafe-eval'; style-src 'self'; img-src 'self' https: data:; font-src 'self' data:; connect-src 'self' https://api.iconify.design https://api.unisvg.com https://api.simplesvg.com; worker-src 'self'",
            ),
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use serde_json::{Value, json};
    use tempfile::{TempDir, tempdir};
    use tower::util::ServiceExt;

    use super::*;

    async fn build_test_router() -> (Router, Database, TempDir) {
        let tempdir = tempdir().expect("tempdir");
        let data_dir = tempdir.path().join("web-data");
        let db = Database::new(data_dir.clone());
        db.initialize().await.expect("initialize db");
        let router = build_router(WebAppState::new(
            db.clone(),
            WebRuntimeConfig {
                listen: "127.0.0.1:17172".parse().expect("socket addr"),
                data_dir,
                tmdb_api_key: None,
                runtime_log_dir: tempdir.path().join("runtime-logs"),
                stream_log_dir: tempdir.path().join("stream-logs"),
                executable_path: tempdir.path().join("embystream"),
                main_config_path: Some(tempdir.path().join("config.toml")),
            },
        ));
        (router, db, tempdir)
    }

    async fn json_body(response: axum::response::Response) -> Value {
        let bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        serde_json::from_slice(&bytes).expect("json")
    }

    async fn login_cookie(
        router: Router,
        username: &str,
        email: &str,
        password: &str,
    ) -> String {
        let register_request = Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "username": username,
                    "email": email,
                    "password": password
                })
                .to_string(),
            ))
            .expect("request");
        let register_response = router
            .clone()
            .oneshot(register_request)
            .await
            .expect("register");
        assert_eq!(register_response.status(), StatusCode::OK);

        let login_request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "login": username,
                    "password": password
                })
                .to_string(),
            ))
            .expect("request");
        let login_response =
            router.oneshot(login_request).await.expect("login");
        assert_eq!(login_response.status(), StatusCode::OK);

        login_response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie header")
            .split(';')
            .next()
            .expect("cookie pair")
            .to_string()
    }

    #[tokio::test]
    async fn admin_bootstrap_creates_default_admin_once() {
        let tempdir = tempdir().expect("tempdir");
        let db = Database::new(tempdir.path().join("web-data"));

        let first = db.initialize().await.expect("first init");
        let second = db.initialize().await.expect("second init");

        assert!(first.is_some(), "first init should bootstrap admin");
        assert!(second.is_none(), "second init should not recreate admin");

        let admin = db
            .find_user_by_login("admin".to_string())
            .await
            .expect("find admin")
            .expect("admin exists");
        assert_eq!(admin.role, crate::web::contracts::UserRole::Admin);
    }

    #[tokio::test]
    async fn invalid_login_returns_unauthorized() {
        let (router, _, _tempdir) = build_test_router().await;

        let register_request = Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "username": "demo-user",
                    "email": "demo@example.com",
                    "password": "super-secret"
                })
                .to_string(),
            ))
            .expect("request");
        let register_response = router
            .clone()
            .oneshot(register_request)
            .await
            .expect("register");
        assert_eq!(register_response.status(), StatusCode::OK);

        let login_request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "login": "demo-user",
                    "password": "wrong-pass"
                })
                .to_string(),
            ))
            .expect("request");
        let login_response =
            router.oneshot(login_request).await.expect("login");

        assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
        let body = json_body(login_response).await;
        assert_eq!(body["error"]["code"], "unauthorized");
    }

    #[tokio::test]
    async fn api_responses_include_security_headers() {
        let (router, _, _tempdir) = build_test_router().await;

        let request = Request::builder()
            .method("GET")
            .uri("/api/auth/me")
            .body(Body::empty())
            .expect("request");
        let response = router.oneshot(request).await.expect("response");

        assert_eq!(
            response.headers().get(header::X_FRAME_OPTIONS),
            Some(&header::HeaderValue::from_static("DENY"))
        );
        assert_eq!(
            response.headers().get(header::X_CONTENT_TYPE_OPTIONS),
            Some(&header::HeaderValue::from_static("nosniff"))
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&header::HeaderValue::from_static("no-store"))
        );
    }

    #[tokio::test]
    async fn cross_origin_unsafe_request_is_blocked() {
        let (router, _, _tempdir) = build_test_router().await;

        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::HOST, "127.0.0.1:17172")
            .header(header::ORIGIN, "https://evil.example")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "login": "admin",
                    "password": "wrong-pass"
                })
                .to_string(),
            ))
            .expect("request");
        let response = router.oneshot(request).await.expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "forbidden");
    }

    #[tokio::test]
    async fn login_rate_limit_blocks_bruteforce_attempts() {
        let (router, _, _tempdir) = build_test_router().await;

        let register_request = Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "username": "ratelimit",
                    "email": "ratelimit@example.com",
                    "password": "correct-pass"
                })
                .to_string(),
            ))
            .expect("request");
        let register_response = router
            .clone()
            .oneshot(register_request)
            .await
            .expect("register");
        assert_eq!(register_response.status(), StatusCode::OK);

        for _ in 0..10 {
            let login_request = Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("x-forwarded-for", "203.0.113.9")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "login": "ratelimit",
                        "password": "wrong-pass"
                    })
                    .to_string(),
                ))
                .expect("request");
            let login_response =
                router.clone().oneshot(login_request).await.expect("login");
            assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);
        }

        let blocked_request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("x-forwarded-for", "203.0.113.9")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "login": "ratelimit",
                    "password": "wrong-pass"
                })
                .to_string(),
            ))
            .expect("request");
        let blocked_response = router
            .oneshot(blocked_request)
            .await
            .expect("blocked login");

        assert_eq!(blocked_response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = json_body(blocked_response).await;
        assert_eq!(body["error"]["code"], "rate_limited");
    }

    #[tokio::test]
    async fn secure_cookie_is_set_when_forwarded_proto_is_https() {
        let (router, _, _tempdir) = build_test_router().await;

        let register_request = Request::builder()
            .method("POST")
            .uri("/api/auth/register")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "username": "securecookie",
                    "email": "securecookie@example.com",
                    "password": "secure-pass"
                })
                .to_string(),
            ))
            .expect("request");
        let register_response = router
            .clone()
            .oneshot(register_request)
            .await
            .expect("register");
        assert_eq!(register_response.status(), StatusCode::OK);

        let login_request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .header("x-forwarded-proto", "https")
            .body(Body::from(
                json!({
                    "login": "securecookie",
                    "password": "secure-pass"
                })
                .to_string(),
            ))
            .expect("request");
        let login_response =
            router.oneshot(login_request).await.expect("login");

        let set_cookie = login_response
            .headers()
            .get(header::SET_COOKIE)
            .expect("set-cookie")
            .to_str()
            .expect("cookie str")
            .to_string();
        assert!(set_cookie.contains("Secure"));
    }

    #[tokio::test]
    async fn change_own_password_requires_current_password_and_invalidates_old_login()
     {
        let (router, _, _tempdir) = build_test_router().await;
        let cookie = login_cookie(
            router.clone(),
            "self-edit",
            "self-edit@example.com",
            "old-pass-123",
        )
        .await;

        let wrong_request = Request::builder()
            .method("PATCH")
            .uri("/api/auth/password")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, cookie.clone())
            .body(Body::from(
                json!({
                    "current_password": "wrong-pass",
                    "new_password": "new-pass-456"
                })
                .to_string(),
            ))
            .expect("request");
        let wrong_response = router
            .clone()
            .oneshot(wrong_request)
            .await
            .expect("wrong response");
        assert_eq!(wrong_response.status(), StatusCode::BAD_REQUEST);

        let change_request = Request::builder()
            .method("PATCH")
            .uri("/api/auth/password")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, cookie.clone())
            .body(Body::from(
                json!({
                    "current_password": "old-pass-123",
                    "new_password": "new-pass-456"
                })
                .to_string(),
            ))
            .expect("request");
        let change_response = router
            .clone()
            .oneshot(change_request)
            .await
            .expect("change password");
        assert_eq!(change_response.status(), StatusCode::OK);

        let old_login_request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "login": "self-edit",
                    "password": "old-pass-123"
                })
                .to_string(),
            ))
            .expect("request");
        let old_login_response = router
            .clone()
            .oneshot(old_login_request)
            .await
            .expect("old login");
        assert_eq!(old_login_response.status(), StatusCode::UNAUTHORIZED);

        let new_login_request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(
                json!({
                    "login": "self-edit",
                    "password": "new-pass-456"
                })
                .to_string(),
            ))
            .expect("request");
        let new_login_response =
            router.oneshot(new_login_request).await.expect("new login");
        assert_eq!(new_login_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn non_admin_cannot_access_logs() {
        let (router, _, _tempdir) = build_test_router().await;

        let cookie = login_cookie(
            router.clone(),
            "viewer",
            "viewer@example.com",
            "viewer-pass",
        )
        .await;

        let logs_request = Request::builder()
            .method("GET")
            .uri("/api/logs")
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .expect("request");
        let logs_response = router.oneshot(logs_request).await.expect("logs");

        assert_eq!(logs_response.status(), StatusCode::FORBIDDEN);
        let body = json_body(logs_response).await;
        assert_eq!(body["error"]["code"], "forbidden");
    }

    #[tokio::test]
    async fn draft_generation_persists_config_sets_and_artifacts() {
        let (router, _, _tempdir) = build_test_router().await;
        let cookie = login_cookie(
            router.clone(),
            "builder",
            "builder@example.com",
            "builder-pass",
        )
        .await;

        let create_request = Request::builder()
            .method("POST")
            .uri("/api/drafts")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, cookie.clone())
            .body(Body::from(
                json!({
                    "name": "Living room setup",
                    "stream_mode": "frontend"
                })
                .to_string(),
            ))
            .expect("request");
        let create_response = router
            .clone()
            .oneshot(create_request)
            .await
            .expect("create draft");
        assert_eq!(create_response.status(), StatusCode::OK);
        let create_body = json_body(create_response).await;
        let draft_id = create_body["draft"]["id"]
            .as_str()
            .expect("draft id")
            .to_string();

        let generate_request = Request::builder()
            .method("POST")
            .uri(format!("/api/drafts/{draft_id}/generate"))
            .header(header::COOKIE, cookie.clone())
            .body(Body::empty())
            .expect("request");
        let generate_response = router
            .clone()
            .oneshot(generate_request)
            .await
            .expect("generate");
        assert_eq!(generate_response.status(), StatusCode::OK);
        let generate_body = json_body(generate_response).await;
        let config_set_id = generate_body["config_set"]["id"]
            .as_str()
            .expect("config set id")
            .to_string();
        assert_eq!(
            generate_body["artifacts"].as_array().map(Vec::len),
            Some(5)
        );

        let list_request = Request::builder()
            .method("GET")
            .uri("/api/config-sets")
            .header(header::COOKIE, cookie.clone())
            .body(Body::empty())
            .expect("request");
        let list_response = router
            .clone()
            .oneshot(list_request)
            .await
            .expect("list config sets");
        assert_eq!(list_response.status(), StatusCode::OK);
        let list_body = json_body(list_response).await;
        assert_eq!(list_body["items"].as_array().map(Vec::len), Some(1));

        let artifacts_request = Request::builder()
            .method("GET")
            .uri(format!("/api/config-sets/{config_set_id}/artifacts"))
            .header(header::COOKIE, cookie)
            .body(Body::empty())
            .expect("request");
        let artifacts_response =
            router.oneshot(artifacts_request).await.expect("artifacts");
        assert_eq!(artifacts_response.status(), StatusCode::OK);
        let artifacts_body = json_body(artifacts_response).await;
        assert_eq!(artifacts_body["items"].as_array().map(Vec::len), Some(5));
    }
}
