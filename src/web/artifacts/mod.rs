use axum::{
    Json, Router,
    extract::{Path, State},
    http::header,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use axum_extra::extract::CookieJar;

use crate::{
    config::types::RawConfig,
    core::backend::webdav::{DEFAULT_QUERY_PARAM, MODE_PATH_JOIN},
    web::{
        api::WebAppState,
        auth::session_user_from_jar,
        contracts::{
            ArtifactDocument, ArtifactSummary, ArtifactType, ConfigSetEnvelope,
            DraftEnvelope, LogoutResponse, MetadataUpdateRequest,
            WizardPayload,
        },
        error::WebError,
    },
};

#[derive(Debug, Clone)]
pub struct RenderedArtifact {
    pub artifact_type: ArtifactType,
    pub file_name: String,
    pub language: String,
    pub content: String,
}

impl RenderedArtifact {
    pub fn summary(&self) -> ArtifactSummary {
        ArtifactSummary {
            artifact_type: self.artifact_type,
            file_name: self.file_name.clone(),
        }
    }

    pub fn document(&self) -> ArtifactDocument {
        ArtifactDocument {
            artifact_type: self.artifact_type,
            file_name: self.file_name.clone(),
            language: self.language.clone(),
            content: self.content.clone(),
        }
    }
}

pub fn routes() -> Router<WebAppState> {
    Router::new()
        .route("/{config_set_id}/artifacts", get(list_artifacts))
        .route(
            "/{config_set_id}/artifacts/{artifact_type}/download",
            get(download_artifact),
        )
        .route("/{config_set_id}/duplicate", post(duplicate_config_set))
        .route(
            "/{config_set_id}/metadata",
            patch(update_config_set_metadata),
        )
        .route("/{config_set_id}", delete(delete_config_set))
}

pub fn render_all(
    raw: &RawConfig,
    payload: &WizardPayload,
    config_toml: String,
) -> Vec<RenderedArtifact> {
    vec![
        RenderedArtifact {
            artifact_type: ArtifactType::ConfigToml,
            file_name: "config.toml".to_string(),
            language: "toml".to_string(),
            content: config_toml,
        },
        RenderedArtifact {
            artifact_type: ArtifactType::NginxConf,
            file_name: "nginx.conf".to_string(),
            language: "nginx".to_string(),
            content: render_nginx_conf(raw, payload),
        },
        RenderedArtifact {
            artifact_type: ArtifactType::DockerCompose,
            file_name: "docker-compose.yaml".to_string(),
            language: "yaml".to_string(),
            content: render_docker_compose(raw),
        },
        RenderedArtifact {
            artifact_type: ArtifactType::SystemdService,
            file_name: "systemd.service".to_string(),
            language: "ini".to_string(),
            content: render_systemd_service(raw, payload),
        },
        RenderedArtifact {
            artifact_type: ArtifactType::Pm2Config,
            file_name: "pm2.config.cjs".to_string(),
            language: "javascript".to_string(),
            content: render_pm2_config(raw, payload),
        },
    ]
}

fn render_nginx_conf(raw: &RawConfig, payload: &WizardPayload) -> String {
    let frontend_port = raw
        .frontend
        .as_ref()
        .map(|frontend| frontend.listen_port)
        .unwrap_or(60001);
    let backend_port = raw
        .backend
        .as_ref()
        .map(|backend| backend.listen_port)
        .unwrap_or(frontend_port);
    let emby_port = raw.emby.port.trim();
    let frontend_nginx = &payload.nginx.frontend;
    let backend_nginx = &payload.nginx.backend;

    let frontend_server_name = normalize_server_name(
        &frontend_nginx.server_name,
        "stream.example.com",
    );
    let backend_server_name = normalize_server_name(
        &backend_nginx.server_name,
        &frontend_server_name,
    );
    let frontend_client_max_body_size =
        normalize_or_default(&frontend_nginx.client_max_body_size, "100M");
    let backend_client_max_body_size =
        normalize_or_default(&backend_nginx.client_max_body_size, "1G");
    let resolver_line = build_backend_resolver_line(
        &backend_nginx.resolver_provider,
        &backend_nginx.custom_resolvers,
    );
    let frontend_ssl_block = render_tls_directives(
        &frontend_nginx.ssl_certificate,
        &frontend_nginx.ssl_certificate_key,
        "    ",
    );
    let backend_ssl_block = render_tls_directives(
        &backend_nginx.ssl_certificate,
        &backend_nginx.ssl_certificate_key,
        "    ",
    );
    let has_google_drive_node = raw
        .backend_nodes
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .any(|node| node.google_drive.is_some());
    let backend_log_formats = if has_google_drive_node {
        r#"log_format main_ext
    '$remote_addr - $remote_user [$time_local] '
    '"$request" $status $body_bytes_sent '
    'rt=$request_time urt=$upstream_response_time '
    'ua="$http_user_agent" ref="$http_referer" '
    'host="$host" upstream="$upstream_addr" ustatus="$upstream_status" '
    'xff="$http_x_forwarded_for" range="$http_range"'
;

log_format google_drive_ext
    '$remote_addr [$time_local] '
    '"$request" $status '
    'rt=$request_time urt=$upstream_response_time '
    'upstream="$upstream_addr" ustatus="$upstream_status" '
    'uri="$uri" args="$args" '
    'range="$http_range" if_range="$http_if_range" '
    'ua="$http_user_agent"'
;

"#
        .to_string()
    } else {
        String::new()
    };
    let backend_access_log = if has_google_drive_node {
        format!(
            "    access_log {} main_ext;",
            normalize_or_default(
                &backend_nginx.access_log_path,
                "/var/log/nginx/embystream_access.log",
            )
        )
    } else {
        format!(
            "    access_log {};",
            normalize_or_default(
                &backend_nginx.access_log_path,
                "/var/log/nginx/embystream_access.log",
            )
        )
    };

    let websocket_location = format!(
        r#"    location ~* {pattern} {{
        proxy_pass http://127.0.0.1:{emby_port};
        proxy_set_header Host $host;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Protocol $scheme;
        proxy_set_header X-Forwarded-Host $http_host;
        proxy_cache off;
    }}"#,
        pattern = normalize_or_default(
            &frontend_nginx.websocket_location_pattern,
            r"/(socket|embywebsocket)",
        ),
        emby_port = if emby_port.is_empty() {
            "8096"
        } else {
            emby_port
        },
    );

    let static_location = format!(
        r#"    location ~* {pattern} {{
        proxy_pass http://127.0.0.1:{frontend_port};
        proxy_set_header Host $host;
        proxy_set_header Connection "upgrade";
        expires 10y;
        add_header Pragma "public";
        add_header Cache-Control "public";
    }}"#,
        pattern = normalize_or_default(
            &frontend_nginx.static_location_pattern,
            r"\.(webp|jpg|jpeg|png|gif|ico|css|js|html)$|Images|fonts",
        ),
    );

    let accel_blocks = render_accel_blocks(raw, payload);

    match raw.general.stream_mode {
        crate::config::general::StreamMode::Frontend => format!(
            r#"server {{
    listen 80;
    listen [::]:80;

    server_name {server_name};
    return 301 https://$host$request_uri;
}}

server {{
    listen 443 ssl;
    listen [::]:443 ssl;
    http2 on;

    server_name {server_name};

    ssl_session_timeout 30m;
    ssl_protocols TLSv1.1 TLSv1.2 TLSv1.3;
{ssl_block}
    ssl_session_cache shared:SSL:10m;

    client_max_body_size {client_max_body_size};

    add_header Referrer-Policy "origin-when-cross-origin";
    add_header Strict-Transport-Security "max-age=15552000; preload" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

{static_location}

{websocket_location}

    location / {{
        proxy_pass http://127.0.0.1:{frontend_port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header Range $http_range;
        proxy_set_header If-Range $http_if_range;
        proxy_hide_header X-Powered-By;

        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $http_connection;

        proxy_buffering off;
    }}
}}
"#,
            server_name = frontend_server_name,
            ssl_block = frontend_ssl_block,
            client_max_body_size = frontend_client_max_body_size,
            static_location = static_location,
            websocket_location = websocket_location,
            frontend_port = frontend_port,
        ),
        crate::config::general::StreamMode::Backend => format!(
            r#"{log_formats}server {{
    listen 80;
    listen [::]:80;

    server_name {server_name};
    return 301 https://$host$request_uri;
}}

server {{
    listen 443 ssl;
    listen [::]:443 ssl;
    http2 on;

    server_name {server_name};

{ssl_block}

    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers 'TLS_AES_128_GCM_SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-AES256-GCM-SHA384';
    ssl_prefer_server_ciphers on;
{resolver_line}

    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;

    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/javascript application/xml+rss application/xml image/svg+xml;

    client_max_body_size {client_max_body_size};
    client_body_buffer_size 128k;
    proxy_buffer_size 128k;
    proxy_buffers 4 256k;
    proxy_busy_buffers_size 256k;

{access_log}
    error_log {error_log_path} info;

    location /{stream_path} {{
        proxy_pass http://127.0.0.1:{backend_port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        proxy_read_timeout 1200s;
        proxy_send_timeout 1200s;
        proxy_connect_timeout 120s;

        proxy_buffering off;
        proxy_request_buffering off;
    }}

    location / {{
        proxy_pass http://127.0.0.1:{backend_port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        proxy_read_timeout 1200s;
        proxy_send_timeout 1200s;
    }}
{accel_blocks}
}}
"#,
            log_formats = backend_log_formats,
            server_name = backend_server_name,
            ssl_block = backend_ssl_block,
            resolver_line = resolver_line,
            client_max_body_size = backend_client_max_body_size,
            access_log = backend_access_log,
            error_log_path = normalize_or_default(
                &backend_nginx.error_log_path,
                "/var/log/nginx/embystream_error.log",
            ),
            stream_path = raw
                .backend
                .as_ref()
                .map(|backend| backend.path.trim_matches('/'))
                .filter(|path| !path.is_empty())
                .unwrap_or("stream"),
            backend_port = backend_port,
            accel_blocks = accel_blocks,
        ),
        crate::config::general::StreamMode::Dual => format!(
            r#"server {{
    listen 80;
    listen [::]:80;

    server_name {server_name};
    return 301 https://$host$request_uri;
}}

server {{
    listen 443 ssl;
    listen [::]:443 ssl;
    http2 on;

    server_name {server_name};

{ssl_block}

    ssl_protocols TLSv1.2 TLSv1.3;
    client_max_body_size {client_max_body_size};

    add_header Referrer-Policy "origin-when-cross-origin";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

{static_location}

{websocket_location}

    location /{stream_path} {{
        proxy_pass http://127.0.0.1:{backend_port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 1200s;
        proxy_send_timeout 1200s;
        proxy_connect_timeout 120s;
        proxy_buffering off;
        proxy_request_buffering off;
    }}

    location / {{
        proxy_pass http://127.0.0.1:{frontend_port};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header Range $http_range;
        proxy_set_header If-Range $http_if_range;
        proxy_hide_header X-Powered-By;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection $http_connection;
        proxy_buffering off;
    }}
{accel_blocks}
}}
"#,
            server_name = frontend_server_name,
            ssl_block = frontend_ssl_block,
            client_max_body_size = frontend_client_max_body_size,
            static_location = static_location,
            websocket_location = websocket_location,
            stream_path = raw
                .backend
                .as_ref()
                .map(|backend| backend.path.trim_matches('/'))
                .filter(|path| !path.is_empty())
                .unwrap_or("stream"),
            frontend_port = frontend_port,
            backend_port = backend_port,
            accel_blocks = accel_blocks,
        ),
    }
}

fn normalize_or_default(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_server_name(value: &str, fallback: &str) -> String {
    normalize_or_default(
        value
            .trim()
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/'),
        fallback,
    )
}

fn render_tls_directives(
    certificate: &str,
    private_key: &str,
    indent: &str,
) -> String {
    let certificate = certificate.trim();
    let private_key = private_key.trim();
    let certificate_line = if certificate.is_empty() {
        format!(
            "{indent}# TODO: replace with your TLS certificate path.\n{indent}# ssl_certificate /path/to/fullchain.pem;"
        )
    } else {
        format!("{indent}ssl_certificate {certificate};")
    };
    let private_key_line = if private_key.is_empty() {
        format!(
            "{indent}# TODO: replace with your TLS private key path.\n{indent}# ssl_certificate_key /path/to/privkey.pem;"
        )
    } else {
        format!("{indent}ssl_certificate_key {private_key};")
    };

    format!("{certificate_line}\n{private_key_line}")
}

fn build_backend_resolver_line(
    provider: &str,
    custom_resolvers: &str,
) -> String {
    let resolvers = match provider.trim() {
        "cloudflare" => Some(
            "1.1.1.1 1.0.0.1 [2606:4700:4700::1111] [2606:4700:4700::1001]",
        ),
        "dnspod" => Some("119.29.29.29 182.254.116.116"),
        "google" => Some(
            "8.8.8.8 8.8.4.4 [2001:4860:4860::8888] [2001:4860:4860::8844]",
        ),
        "aliyun" => {
            Some("223.5.5.5 223.6.6.6 [2400:3200::1] [2400:3200:baba::1]")
        }
        "tencent" => Some("119.28.28.28 182.254.118.118"),
        "custom" => {
            let custom = custom_resolvers.trim();
            if custom.is_empty() {
                None
            } else {
                Some(custom)
            }
        }
        "none" => None,
        "godaddy" => {
            let custom = custom_resolvers.trim();
            if custom.is_empty() {
                None
            } else {
                Some(custom)
            }
        }
        _ => {
            let custom = custom_resolvers.trim();
            if custom.is_empty() {
                None
            } else {
                Some(custom)
            }
        }
    };

    match resolvers {
        Some(resolvers) => format!(
            "\n    resolver {resolvers} valid=300s;\n    resolver_timeout 30s;"
        ),
        None => String::new(),
    }
}

fn render_accel_blocks(raw: &RawConfig, payload: &WizardPayload) -> String {
    let mut accel_blocks = String::new();
    let backend_google_access_log = normalize_or_default(
        &payload.nginx.backend.google_drive_access_log_path,
        "/var/log/nginx/google_drive_access.log",
    );

    for node in raw.backend_nodes.as_deref().unwrap_or(&[]) {
        if let Some(webdav) = node.webdav.as_ref() {
            let node_uuid = webdav.node_uuid.trim();
            if !node_uuid.is_empty()
                && node.proxy_mode.eq_ignore_ascii_case("accel_redirect")
            {
                let path = node.path.trim_matches('/');
                let upstream_path = if path.is_empty() {
                    String::new()
                } else {
                    format!("/{path}")
                };
                let port = if node.port.is_empty() {
                    "80".to_string()
                } else {
                    node.port.clone()
                };
                let host_header = format!(
                    "{}:{}",
                    node.base_url
                        .trim_start_matches("http://")
                        .trim_start_matches("https://")
                        .trim_end_matches('/'),
                    port
                );
                accel_blocks.push_str(&format!(
                    r#"

    location ^~ /_origin/webdav/{node_uuid}/ {{
        internal;
        rewrite ^/_origin/webdav/{node_uuid}/(.*)$ /$1 break;
        proxy_pass {base_url}:{port}{upstream_path};
        proxy_http_version 1.1;
        proxy_set_header Host {host_header};
        proxy_set_header Range $http_range;
        proxy_set_header If-Range $http_if_range;
        proxy_buffering off;
        proxy_request_buffering off;
    }}"#,
                    base_url = node.base_url.trim_end_matches('/'),
                    host_header = host_header,
                ));
            }
        }

        if let Some(google_drive) = node.google_drive.as_ref() {
            if !google_drive.node_uuid.trim().is_empty()
                && node.proxy_mode.eq_ignore_ascii_case("accel_redirect")
            {
                accel_blocks.push_str(&format!(
                    r#"

    location ~ ^/_origin/google-drive/{node_uuid}/([^/]+)/([^/]+)$ {{
        internal;

        access_log {backend_google_access_log} google_drive_ext;

        set $google_node_uuid $1;
        set $google_file_id $2;
        set $google_drive_query "alt=media&supportsAllDrives=true&acknowledgeAbuse=true";

        proxy_pass https://www.googleapis.com/drive/v3/files/$google_file_id?$google_drive_query;
        proxy_http_version 1.1;
        proxy_set_header Authorization "Bearer $arg_token";
        proxy_set_header Host www.googleapis.com;
        proxy_set_header Range $http_range;
        proxy_set_header If-Range $http_if_range;
        proxy_ssl_server_name on;

        proxy_buffering off;
        proxy_request_buffering off;
        proxy_max_temp_file_size 0;

        proxy_read_timeout 3600s;
        proxy_send_timeout 3600s;
        proxy_connect_timeout 60s;
        send_timeout 3600s;
    }}"#,
                    node_uuid = google_drive.node_uuid.trim(),
                    backend_google_access_log = backend_google_access_log,
                ));
            }
        }
    }

    accel_blocks
}

fn render_docker_compose(raw: &RawConfig) -> String {
    let mut ports = Vec::new();
    if let Some(frontend) = raw.frontend.as_ref() {
        ports.push(format!("      - \"{0}:{0}\"", frontend.listen_port));
    }
    if let Some(backend) = raw.backend.as_ref() {
        if ports
            .iter()
            .all(|entry| !entry.contains(&backend.listen_port.to_string()))
        {
            ports.push(format!("      - \"{0}:{0}\"", backend.listen_port));
        }
    }

    format!(
        r#"services:
  embystream:
    image: openpilipili/embystream:latest
    container_name: ${{CONTAINER_NAME:-embystream}}
    environment:
      - TZ=Asia/Shanghai
      - PUID=1000
      - PGID=1000
      - UMASK=022
    volumes:
      # Replace this mount with your real config.toml and edit it as needed.
      - ./config/config.toml:/config/embystream/config.toml
    restart: unless-stopped
    ports:
{ports}
    logging:
      driver: "json-file"
      options:
        max-size: "50m"
        max-file: "3"
"#,
        ports = ports.join("\n")
    )
}

fn render_systemd_service(raw: &RawConfig, payload: &WizardPayload) -> String {
    let description = match raw.general.stream_mode {
        crate::config::general::StreamMode::Frontend => {
            "EmbyStream Frontend Service"
        }
        crate::config::general::StreamMode::Backend => {
            "EmbyStream Backend Service"
        }
        crate::config::general::StreamMode::Dual => {
            "EmbyStream Dual Gateway Service"
        }
    };
    let binary_path = normalize_or_default(
        &payload.deployment.systemd.binary_path,
        "/usr/bin/embystream",
    );
    let working_directory = normalize_or_default(
        &payload.deployment.systemd.working_directory,
        "/opt/stream",
    );
    let config_path = normalize_or_default(
        &payload.deployment.systemd.config_path,
        "/opt/stream/config.toml",
    );
    let web_listen = "0.0.0.0:6888";
    let web_data_dir = format!("{working_directory}/web-config/data");
    let web_runtime_log_dir = format!("{working_directory}/web-config/logs");

    format!(
        r#"[Unit]
Description={description}

[Service]
Type=simple
ExecStartPre=/bin/sleep 3
# If `embystream` is installed elsewhere, run `which embystream` and update
# the binary path below.
ExecStart={binary_path} run --config {config_path} --web --web-listen {web_listen} --web-data-dir {web_data_dir} --web-runtime-log-dir {web_runtime_log_dir}
# Update these paths if your deployment lives outside the default directory.
WorkingDirectory={working_directory}
User=root
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
"#,
        binary_path = binary_path,
        config_path = config_path,
        web_listen = web_listen,
        web_data_dir = web_data_dir,
        web_runtime_log_dir = web_runtime_log_dir,
        working_directory = working_directory,
    )
}

fn render_pm2_config(raw: &RawConfig, payload: &WizardPayload) -> String {
    let app_name = match raw.general.stream_mode {
        crate::config::general::StreamMode::Frontend => "stream-frontend",
        crate::config::general::StreamMode::Backend => "stream-backend",
        crate::config::general::StreamMode::Dual => "stream",
    };
    let default_working_directory = match raw.general.stream_mode {
        crate::config::general::StreamMode::Frontend => "/opt/stream-frontend",
        crate::config::general::StreamMode::Backend => "/opt/stream-backend",
        crate::config::general::StreamMode::Dual => "/opt/stream",
    };
    let binary_path = normalize_or_default(
        &payload.deployment.pm2.binary_path,
        "/usr/bin/embystream",
    );
    let working_directory = normalize_or_default(
        &payload.deployment.pm2.working_directory,
        default_working_directory,
    );
    let config_path = normalize_or_default(
        &payload.deployment.pm2.config_path,
        &format!("{working_directory}/config.toml"),
    );
    let out_file = normalize_or_default(
        &payload.deployment.pm2.out_file,
        &format!("{working_directory}/logs/pm2.out.log"),
    );
    let error_file = normalize_or_default(
        &payload.deployment.pm2.error_file,
        &format!("{working_directory}/logs/pm2.err.log"),
    );
    let web_listen = "0.0.0.0:6888";
    let web_data_dir = format!("{working_directory}/web-config/data");
    let web_runtime_log_dir = format!("{working_directory}/web-config/logs");

    format!(
        r#"module.exports = {{
  apps: [
    {{
      name: "{app_name}",
      // If `embystream` is installed elsewhere, run `which embystream` and
      // update the script path below.
      script: "{binary_path}",
      // Update these paths if your deployment lives outside the default
      // directory for this mode.
      cwd: "{working_directory}",
      args: ["run", "--config", "{config_path}", "--web", "--web-listen", "{web_listen}", "--web-data-dir", "{web_data_dir}", "--web-runtime-log-dir", "{web_runtime_log_dir}"],
      out_file: "{out_file}",
      error_file: "{error_file}",
      time: true,
      env: {{
        TZ: "Asia/Shanghai"
      }}
    }}
  ]
}};
"#,
        binary_path = binary_path,
        working_directory = working_directory,
        config_path = config_path,
        web_listen = web_listen,
        web_data_dir = web_data_dir,
        web_runtime_log_dir = web_runtime_log_dir,
        out_file = out_file,
        error_file = error_file,
    )
}

#[allow(dead_code)]
fn _webdav_defaults() -> (&'static str, &'static str) {
    (MODE_PATH_JOIN, DEFAULT_QUERY_PARAM)
}

async fn list_artifacts(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(config_set_id): Path<String>,
) -> Result<Json<super::contracts::ArtifactListResponse>, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    let artifacts = state
        .db
        .list_config_set_artifacts(&user.id, &config_set_id)
        .await?
        .ok_or(WebError::NotFound("Generated config set was not found."))?;
    Ok(Json(artifacts))
}

async fn duplicate_config_set(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(config_set_id): Path<String>,
) -> Result<Json<DraftEnvelope>, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    let draft = state
        .db
        .duplicate_config_set(&user.id, &config_set_id)
        .await?
        .ok_or(WebError::NotFound("Generated config set was not found."))?;
    Ok(Json(DraftEnvelope { draft }))
}

async fn delete_config_set(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(config_set_id): Path<String>,
) -> Result<Json<LogoutResponse>, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    state.db.delete_config_set(&user.id, &config_set_id).await?;
    Ok(Json(LogoutResponse { ok: true }))
}

async fn update_config_set_metadata(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path(config_set_id): Path<String>,
    Json(payload): Json<MetadataUpdateRequest>,
) -> Result<Json<ConfigSetEnvelope>, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    let name = payload.name.trim().to_string();
    if name.is_empty() {
        return Err(WebError::invalid_input(
            "name",
            "Config set name is required.",
        ));
    }

    let config_set = state
        .db
        .update_config_set_metadata(&user.id, &config_set_id, name)
        .await?
        .ok_or(WebError::NotFound("Generated config set was not found."))?;

    Ok(Json(ConfigSetEnvelope { config_set }))
}

async fn download_artifact(
    State(state): State<WebAppState>,
    jar: CookieJar,
    Path((config_set_id, artifact_type)): Path<(String, String)>,
) -> Result<impl IntoResponse, WebError> {
    let user = session_user_from_jar(&state, &jar).await?;
    let artifact_type = parse_artifact_type_param(&artifact_type)
        .ok_or(WebError::NotFound("Artifact type was not found."))?;
    let artifacts = state
        .db
        .list_config_set_artifacts(&user.id, &config_set_id)
        .await?
        .ok_or(WebError::NotFound("Generated config set was not found."))?;
    let artifact = artifacts
        .items
        .into_iter()
        .find(|item| item.artifact_type == artifact_type)
        .ok_or(WebError::NotFound("Artifact was not found."))?;

    Ok((
        [(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", artifact.file_name),
        )],
        artifact.content,
    ))
}

fn parse_artifact_type_param(value: &str) -> Option<ArtifactType> {
    match value {
        "config_toml" => Some(ArtifactType::ConfigToml),
        "nginx_conf" => Some(ArtifactType::NginxConf),
        "docker_compose" => Some(ArtifactType::DockerCompose),
        "systemd_service" => Some(ArtifactType::SystemdService),
        "pm2_config" => Some(ArtifactType::Pm2Config),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        cli_wizard::{
            emit::emit_wizard_config_toml, template_payload::build_template_raw,
        },
        config::general::StreamMode,
        web::drafts::wizard_payload_from_raw,
    };

    use super::{
        render_all, render_nginx_conf, render_pm2_config,
        render_systemd_service,
    };

    #[test]
    fn render_all_emits_every_required_artifact() {
        let raw = build_template_raw(StreamMode::Dual);
        let config_toml = emit_wizard_config_toml(&raw).expect("toml");
        let payload = wizard_payload_from_raw(raw.clone());
        let artifacts = render_all(&raw, &payload, config_toml);

        assert_eq!(artifacts.len(), 5);
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.file_name == "config.toml")
        );
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.file_name == "nginx.conf")
        );
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.file_name == "docker-compose.yaml")
        );
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.file_name == "systemd.service")
        );
        assert!(
            artifacts
                .iter()
                .any(|artifact| artifact.file_name == "pm2.config.cjs")
        );
    }

    #[test]
    fn rendered_artifacts_use_runtime_values() {
        let raw = build_template_raw(StreamMode::Dual);
        let frontend_port = raw
            .frontend
            .as_ref()
            .expect("frontend")
            .listen_port
            .to_string();
        let backend_port = raw
            .backend
            .as_ref()
            .expect("backend")
            .listen_port
            .to_string();

        let config_toml = emit_wizard_config_toml(&raw).expect("toml");
        let payload = wizard_payload_from_raw(raw.clone());
        let artifacts = render_all(&raw, &payload, config_toml);

        let nginx = artifacts
            .iter()
            .find(|artifact| artifact.file_name == "nginx.conf")
            .expect("nginx");
        let docker_compose = artifacts
            .iter()
            .find(|artifact| artifact.file_name == "docker-compose.yaml")
            .expect("compose");
        let pm2 = artifacts
            .iter()
            .find(|artifact| artifact.file_name == "pm2.config.cjs")
            .expect("pm2");

        assert!(nginx.content.contains(&frontend_port));
        assert!(docker_compose.content.contains(&frontend_port));
        assert!(docker_compose.content.contains(&backend_port));
        assert!(docker_compose.content.contains(
            "# Replace this mount with your real config.toml and edit it as needed."
        ));
        assert!(pm2.content.contains("/opt/stream/config.toml"));
        assert!(pm2.content.contains("Asia/Shanghai"));
    }

    #[test]
    fn backend_nginx_omits_google_drive_log_formats_without_google_drive_node()
    {
        let mut raw = build_template_raw(StreamMode::Backend);
        raw.backend_nodes = Some(vec![]);
        let payload = wizard_payload_from_raw(raw.clone());

        let nginx = render_nginx_conf(&raw, &payload);

        assert!(!nginx.contains("log_format main_ext"));
        assert!(!nginx.contains("log_format google_drive_ext"));
        assert!(
            nginx.contains("access_log /var/log/nginx/embystream_access.log;")
        );
    }

    #[test]
    fn backend_nginx_adds_tls_comments_and_google_drive_log_formats() {
        let raw = build_template_raw(StreamMode::Backend);
        let payload = wizard_payload_from_raw(raw.clone());

        let nginx = render_nginx_conf(&raw, &payload);

        assert!(nginx.contains("log_format main_ext"));
        assert!(nginx.contains("log_format google_drive_ext"));
        assert!(
            nginx.contains("# TODO: replace with your TLS certificate path.")
        );
        assert!(nginx.contains("# ssl_certificate /path/to/fullchain.pem;"));
        assert!(nginx.contains(
            "access_log /var/log/nginx/embystream_access.log main_ext;"
        ));
    }

    #[test]
    fn systemd_and_pm2_use_new_default_paths_without_pilipili() {
        let raw = build_template_raw(StreamMode::Backend);
        let payload = wizard_payload_from_raw(raw.clone());

        let systemd = render_systemd_service(&raw, &payload);
        let pm2 = render_pm2_config(&raw, &payload);

        assert!(systemd.contains(
            "ExecStart=/usr/bin/embystream run --config /opt/stream/config.toml --web --web-listen 0.0.0.0:6888 --web-data-dir /opt/stream/web-config/data --web-runtime-log-dir /opt/stream/web-config/logs"
        ));
        assert!(systemd.contains("WorkingDirectory=/opt/stream"));
        assert!(pm2.contains("name: \"stream-backend\""));
        assert!(pm2.contains("cwd: \"/opt/stream-backend\""));
        assert!(pm2.contains(
            "args: [\"run\", \"--config\", \"/opt/stream-backend/config.toml\", \"--web\", \"--web-listen\", \"0.0.0.0:6888\", \"--web-data-dir\", \"/opt/stream-backend/web-config/data\", \"--web-runtime-log-dir\", \"/opt/stream-backend/web-config/logs\"]"
        ));
        assert!(!systemd.to_lowercase().contains("pilipili"));
        assert!(!pm2.to_lowercase().contains("pilipili"));
    }
}
