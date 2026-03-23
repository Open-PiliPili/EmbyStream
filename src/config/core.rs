use std::{
    fs,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::{Path, PathBuf},
    process,
};

use directories::BaseDirs;
use libc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{
    backend::{Backend, BackendNode},
    error::ConfigError,
    frontend::Frontend,
    general::{General, StreamMode, UserAgent},
    http2::Http2,
    types::{FallbackConfig, RawConfig},
};
use crate::{
    CONFIG_LOGGER_DOMAIN,
    cli::RunArgs,
    config::general::{Log, types::Emby},
    config_error_log, config_info_log, config_warn_log,
    core::backend::constants::{
        STREAM_RELAY_BACKEND_TYPE, backend_base_url_is_empty,
        backend_base_url_is_local_host,
    },
    util::path_rewriter::PathRewriter,
};

const CONFIG_DIR_NAME: &str = "embystream";
const CONFIG_FILE_NAME: &str = "config.toml";
const SSL_DIR_NAME: &str = "ssl";
const SSL_CER_FILE_NAME: &str = "ssl-cert";
const SSL_KEY_FILE_NAME: &str = "ssl-key";
const DOCKER_CONFIG_PATH: &str = "/config/embystream/config.toml";
const TEMPLATE_CONFIG_PATH: &str = "src/config.toml.template";
const ROOT_CONFIG_PATH: &str = "/root/.config/embystream";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip)]
    pub path: PathBuf,
    pub log: Log,
    pub general: General,
    pub emby: Emby,
    pub user_agent: UserAgent,
    pub frontend: Option<Frontend>,
    pub backend: Option<Backend>,
    pub backend_nodes: Vec<BackendNode>,
    pub http2: Http2,
    pub fallback: FallbackConfig,
}

impl Config {
    pub fn load_or_init(args: &RunArgs) -> Result<Self, ConfigError> {
        let config_path = match &args.config {
            Some(path) => path.clone(),
            None => Self::get_default_config_path()?.join(CONFIG_FILE_NAME),
        };

        if !config_path.exists() {
            Self::handle_missing_config(&config_path)?;
            process::exit(1);
        }

        config_info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Loading config file from: {}",
            config_path.display()
        );

        let mut config =
            Self::load_from_path(&config_path).unwrap_or_else(|e| {
                config_error_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "Failed to load or parse config file at '{}': {}",
                    config_path.display(),
                    e
                );
                process::exit(1);
            });

        if let Some(cert_path) = &args.ssl_cert_file {
            config.http2.ssl_cert_file =
                cert_path.to_string_lossy().to_string();
        }
        if let Some(key_path) = &args.ssl_key_file {
            config.http2.ssl_key_file = key_path.to_string_lossy().to_string();
        }

        Ok(config)
    }

    pub fn get_ssl_cert_path(&self) -> Option<PathBuf> {
        let config_dir = self.path.parent()?;
        let path_str = &self.http2.ssl_cert_file;

        if path_str.is_empty() {
            return Some(config_dir.join(SSL_DIR_NAME).join(SSL_CER_FILE_NAME));
        }

        let path = PathBuf::from(path_str);
        if path.is_absolute() {
            Some(path)
        } else {
            Some(config_dir.join(path))
        }
    }

    pub fn get_ssl_key_path(&self) -> Option<PathBuf> {
        let config_dir = self.path.parent()?;
        let path_str = &self.http2.ssl_key_file;

        if path_str.is_empty() {
            return Some(config_dir.join(SSL_DIR_NAME).join(SSL_KEY_FILE_NAME));
        }

        let path = PathBuf::from(path_str);
        if path.is_absolute() {
            Some(path)
        } else {
            Some(config_dir.join(path))
        }
    }

    fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let raw_config: RawConfig = toml::from_str(&content)?;

        let stream_mode = &raw_config.general.stream_mode;

        if (stream_mode == &StreamMode::Frontend
            || stream_mode == &StreamMode::Dual)
            && raw_config.frontend.is_none()
        {
            return Err(ConfigError::MissingConfig("Frontend".to_string()));
        }

        if (stream_mode == &StreamMode::Backend
            || stream_mode == &StreamMode::Dual)
            && raw_config.backend.is_none()
        {
            return Err(ConfigError::MissingConfig("Backend".to_string()));
        }

        let mut backend_nodes = raw_config.backend_nodes.unwrap_or_default();
        for node in &mut backend_nodes {
            node.uuid = Uuid::new_v4().to_string();

            if !node.pattern.is_empty() {
                node.pattern_regex = Some(
                    Regex::new(&node.pattern)
                        .map_err(ConfigError::InvalidRegex)?,
                );
            }

            node.path_rewriter_cache = node
                .path_rewrites
                .iter()
                .map(|pr| {
                    PathRewriter::new(pr.enable, &pr.pattern, &pr.replacement)
                })
                .collect();
        }

        backend_nodes.retain(|node| {
            let drop_relay = node
                .backend_type
                .eq_ignore_ascii_case(STREAM_RELAY_BACKEND_TYPE)
                && (backend_base_url_is_empty(&node.base_url)
                    || backend_base_url_is_local_host(&node.base_url));
            if drop_relay {
                config_warn_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "StreamRelay node '{}' dropped: base_url is empty or loopback \
                     (127.0.0.1, localhost, 0.0.0.0); fix or remove the entry",
                    node.name
                );
            }
            !drop_relay
        });

        Ok(Config {
            path: path.to_path_buf(),
            log: raw_config.log,
            general: raw_config.general,
            emby: raw_config.emby,
            user_agent: raw_config.user_agent,
            frontend: raw_config.frontend,
            backend: raw_config.backend,
            backend_nodes,
            http2: raw_config.http2.unwrap_or_default(),
            fallback: raw_config.fallback,
        })
    }

    fn get_default_config_path() -> Result<PathBuf, ConfigError> {
        if Path::new(DOCKER_CONFIG_PATH).exists() {
            return Ok(PathBuf::from(DOCKER_CONFIG_PATH));
        }

        let base_dirs = BaseDirs::new().ok_or(ConfigError::NoHomeDir)?;

        let path =
            if cfg!(target_os = "linux") && unsafe { libc::getuid() } == 0 {
                PathBuf::from(ROOT_CONFIG_PATH)
            } else if cfg!(target_os = "windows") {
                base_dirs.config_dir().join(CONFIG_DIR_NAME)
            } else {
                base_dirs.home_dir().join(".config").join(CONFIG_DIR_NAME)
            };

        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| ConfigError::CreateDir {
                path: path.display().to_string(),
                source: e,
            })?;

            let ssl_path = path.join(SSL_DIR_NAME);
            fs::create_dir_all(&ssl_path).map_err(|e| {
                ConfigError::CreateDir {
                    path: ssl_path.display().to_string(),
                    source: e,
                }
            })?;
        }

        Ok(path)
    }

    fn handle_missing_config(target_path: &Path) -> Result<(), ConfigError> {
        let template_path = Path::new(TEMPLATE_CONFIG_PATH);

        if !template_path.exists() {
            config_error_log!(
                CONFIG_LOGGER_DOMAIN,
                "Missing template config file at {}",
                template_path.display()
            );
            return Err(ConfigError::CopyTemplate(IoError::new(
                IoErrorKind::NotFound,
                format!(
                    "Template file not found at {}",
                    template_path.display()
                ),
            )));
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::CreateDir {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        fs::copy(template_path, target_path)
            .map_err(ConfigError::CopyTemplate)?;

        config_info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Created new config file at {} from template",
            target_path.display()
        );

        config_error_log!(
            CONFIG_LOGGER_DOMAIN,
            "Please configure the new file and restart the application"
        );
        process::exit(0);
    }
}
