use std::{
    env,
    fs::{OpenOptions, create_dir_all},
    io::Write,
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
};

use axum::Router;
use time::{OffsetDateTime, UtcOffset, macros::format_description};
use tokio::net::TcpListener;

use crate::cli::WebServeArgs;

use super::{
    api::{WebAppState, build_router},
    db::Database,
    error::WebError,
};

#[derive(Debug, Clone)]
pub struct WebRuntimeConfig {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    pub tmdb_api_key: Option<String>,
    pub runtime_log_dir: PathBuf,
    pub stream_log_dir: PathBuf,
    pub executable_path: PathBuf,
    pub main_config_path: Option<PathBuf>,
}

pub fn to_runtime_config(
    args: WebServeArgs,
) -> Result<WebRuntimeConfig, WebError> {
    let listen_value = env::var("WEB_LISTEN")
        .ok()
        .or_else(|| env::var("web_listen").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(args.listen);
    let mut listen = SocketAddr::from_str(&listen_value).map_err(|_| {
        WebError::invalid_input("listen", "Web listen address is invalid.")
    })?;
    if let Some(port) = env::var("webui_port")
        .ok()
        .or_else(|| env::var("WEBUI_PORT").ok())
        .filter(|value| !value.trim().is_empty())
    {
        let port = u16::from_str(port.trim()).map_err(|_| {
            WebError::invalid_input("webui_port", "Web UI port is invalid.")
        })?;
        listen.set_port(port);
    }
    let data_dir = env::var("WEB_DATA_DIR")
        .ok()
        .or_else(|| env::var("web_data_dir").ok())
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(args.data_dir);
    let runtime_log_dir = env::var("WEB_RUNTIME_LOG_DIR")
        .ok()
        .or_else(|| env::var("web_runtime_log_dir").ok())
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(args.runtime_log_dir);
    let stream_log_dir = env::var("WEB_STREAM_LOG_DIR")
        .ok()
        .or_else(|| env::var("web_stream_log_dir").ok())
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(args.stream_log_dir);
    let tmdb_api_key = env::var("TMDB_API_KEY")
        .ok()
        .or_else(|| env::var("tmdb_api_key").ok())
        .filter(|value| !value.trim().is_empty())
        .or(args.tmdb_api_key);

    Ok(WebRuntimeConfig {
        listen,
        data_dir,
        tmdb_api_key: tmdb_api_key
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        runtime_log_dir,
        stream_log_dir,
        executable_path: std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("embystream")),
        main_config_path: None,
    })
}

pub async fn serve_web_app(config: WebRuntimeConfig) -> Result<(), WebError> {
    let listen = config.listen;
    let db = Database::new(config.data_dir.clone());
    let bootstrap = db.initialize().await?;

    if let Some(admin) = bootstrap {
        print_web_startup_line(
            Some((
                &config.runtime_log_dir,
                format!("Bootstrap admin password generated for '{}'.", admin.username),
            )),
            "WARN",
            format!(
                "Bootstrap admin password for '{}': {}",
                admin.username, admin.password
            ),
        );
    }

    let runtime_log_dir = config.runtime_log_dir.clone();
    let state = WebAppState::new(db, config);
    let router: Router = build_router(state);

    let listener = TcpListener::bind(listen).await.map_err(WebError::from)?;
    print_web_startup_line(
        Some((
            &runtime_log_dir,
            format!("Web studio listening on http://{listen}"),
        )),
        "INFO",
        format!("Web studio listening on http://{listen}"),
    );

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .map_err(WebError::from)
}

fn print_web_startup_line(
    persisted: Option<(&Path, String)>,
    level: &str,
    message: String,
) {
    let timer_fmt = format_description!(
        "[year]-[month padding:zero]-[day padding:zero] \
         [hour]:[minute]:[second].[subsecond digits:6]"
    );
    let time_offset =
        UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    let timestamp = OffsetDateTime::now_utc()
        .to_offset(time_offset)
        .format(&timer_fmt)
        .unwrap_or_else(|_| "0000-00-00 00:00:00.000000".to_string());

    let line = format!("{timestamp} {level:<5} [WEB] {message}");
    println!("{line}");

    if let Some((directory, persisted_message)) = persisted {
        let persisted_line =
            format!("{timestamp} {level:<5} [WEB] {persisted_message}");
        if let Err(error) = append_runtime_log_line(directory, &persisted_line)
        {
            eprintln!("Failed to append web runtime log: {error}");
        }
    }
}

fn append_runtime_log_line(directory: &Path, line: &str) -> Result<(), std::io::Error> {
    create_dir_all(directory)?;

    let path = directory.join("runtime.log");
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{line}")?;
    Ok(())
}
