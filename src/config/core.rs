use std::{
    fs,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    path::{Path, PathBuf},
    process,
};

use directories::BaseDirs;
use libc;

use regex::Regex;
use serde::Deserialize;

use super::{
    backend::{Backend, BackendConfig},
    error::ConfigError,
    frontend::Frontend,
    general::{General, StreamMode, UserAgent},
    http2::Http2,
    types::{FallbackConfig, RawConfig},
};
use crate::cli::RunArgs;
use crate::config::general::{Log, types::Emby};
use crate::core::backend::types::{BackendRoute, BackendRoutes};
use crate::util::resolve_fallback_video_path;
use crate::{CONFIG_LOGGER_DOMAIN, config_error_log, config_info_log};

const CONFIG_DIR_NAME: &str = "embystream";
const CONFIG_FILE_NAME: &str = "config.toml";
const SSL_DIR_NAME: &str = "ssl";
const SSL_CER_FILE_NAME: &str = "ssl-cert";
const SSL_KEY_FILE_NAME: &str = "ssl-key";
const DOCKER_CONFIG_PATH: &str = "/config/embystream/config.toml";
const TEMPLATE_CONFIG_PATH: &str = "src/config.toml.template";
const ROOT_CONFIG_PATH: &str = "/root/.config/embystream";

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub path: PathBuf,
    pub log: Log,
    pub general: General,
    pub emby: Emby,
    pub user_agent: UserAgent,
    pub frontend: Option<Frontend>,
    pub backend: Option<Backend>,
    /// Legacy single backend config (for backward compatibility)
    pub backend_config: Option<BackendConfig>,
    /// Backend routes configuration (new routing system)
    #[serde(skip)]
    pub backend_routes: Option<BackendRoutes>,
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

        let needs_backend = stream_mode == &StreamMode::Frontend
            || stream_mode == &StreamMode::Backend
            || stream_mode == &StreamMode::Dual;

        let (backend_config, backend_routes) = if needs_backend {
            if raw_config.backend.is_none() {
                return Err(ConfigError::MissingConfig("Backend".to_string()));
            }

            // Check if routing configuration is provided
            if !raw_config.backend_routes.is_empty() {
                // Use new routing system
                let routes = Self::build_backend_routes(&raw_config, path)?;
                (None, Some(routes))
            } else {
                // Backward compatibility: use legacy single backend_type
                let backend_type = raw_config.general.backend_type.as_str();

                let config = match backend_type.to_lowercase().as_str() {
                    "disk" => {
                        Ok(BackendConfig::Disk(raw_config.disk.ok_or_else(
                            || ConfigError::MissingConfig("Disk".to_string()),
                        )?))
                    }
                    "openlist" => Ok(BackendConfig::OpenList(
                        raw_config.open_list.ok_or_else(|| {
                            ConfigError::MissingConfig("OpenList".to_string())
                        })?,
                    )),
                    "direct_link" => Ok(BackendConfig::DirectLink(
                        raw_config.direct_link.ok_or_else(|| {
                            ConfigError::MissingConfig("DirectLink".to_string())
                        })?,
                    )),
                    other => {
                        Err(ConfigError::InvalidBackendType(other.to_string()))
                    }
                };

                (Some(config?), None)
            }
        } else {
            (None, None)
        };

        Ok(Config {
            path: path.to_path_buf(),
            log: raw_config.log,
            general: raw_config.general,
            emby: raw_config.emby,
            user_agent: raw_config.user_agent,
            frontend: raw_config.frontend,
            backend: raw_config.backend,
            backend_config,
            backend_routes,
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
                // macOS and other Unix-like systems
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

    /// Build backend routes from raw configuration
    fn build_backend_routes(
        raw_config: &RawConfig,
        config_path: &Path,
    ) -> Result<BackendRoutes, ConfigError> {
        // Get routing configuration with defaults
        let routing_config =
            raw_config.backend_routing.clone().unwrap_or_else(|| {
                crate::config::backend::types::BackendRoutingConfig {
                    match_before_rewrite: false,
                    match_priority: "first".to_string(),
                }
            });

        let mut routes = Vec::new();

        // Build route rules
        for route_config in &raw_config.backend_routes {
            // Validate regex pattern
            Regex::new(&route_config.pattern).map_err(|e| {
                ConfigError::InvalidRegex {
                    pattern: route_config.pattern.clone(),
                    error: e.to_string(),
                }
            })?;

            // Get backend config by type
            let backend_config_enum = Self::get_backend_config_by_type(
                &route_config.backend_type,
                raw_config,
            )?;

            // Build BackendConfig with backend settings
            let backend_config = Self::build_backend_config_instance(
                raw_config,
                config_path,
                backend_config_enum,
            );

            routes.push(BackendRoute {
                pattern: route_config.pattern.clone(),
                regex: tokio::sync::OnceCell::new(),
                backend_config,
            });
        }

        // Build fallback configuration
        let fallback_config_enum = if let Some(fallback) =
            &raw_config.backend_fallback
        {
            Self::get_backend_config_by_type(
                &fallback.backend_type,
                raw_config,
            )?
        } else if let Some(first_route) = raw_config.backend_routes.first() {
            // If no fallback configured, use the first route's backend type
            Self::get_backend_config_by_type(
                &first_route.backend_type,
                raw_config,
            )?
        } else {
            // If no routes at all, this should not happen (checked before calling this function)
            return Err(ConfigError::MissingConfig(
                "Backend.Routes (at least one route required)".to_string(),
            ));
        };

        let fallback_config = Self::build_backend_config_instance(
            raw_config,
            config_path,
            fallback_config_enum,
        );

        Ok(BackendRoutes {
            routes,
            fallback: fallback_config,
            match_before_rewrite: routing_config.match_before_rewrite,
            match_priority_first: routing_config.match_priority == "first",
        })
    }

    /// Build a BackendConfig instance from raw config and backend config enum
    fn build_backend_config_instance(
        raw_config: &RawConfig,
        config_path: &Path,
        backend_config_enum: BackendConfig,
    ) -> crate::core::backend::types::BackendConfig {
        let backend =
            raw_config.backend.as_ref().expect("Backend config missing");
        let fallback_video_path = resolve_fallback_video_path(
            &raw_config.fallback.video_missing_path,
            config_path,
        );

        crate::core::backend::types::BackendConfig {
            crypto_key: raw_config.general.encipher_key.clone(),
            crypto_iv: raw_config.general.encipher_iv.clone(),
            backend: backend.clone(),
            backend_config: backend_config_enum,
            fallback_video_path,
        }
    }

    /// Get backend config enum by backend type string
    fn get_backend_config_by_type(
        backend_type: &str,
        raw_config: &RawConfig,
    ) -> Result<BackendConfig, ConfigError> {
        match backend_type.to_lowercase().as_str() {
            "disk" => {
                let disk = raw_config.disk.as_ref().ok_or_else(|| {
                    ConfigError::MissingConfig("Disk".to_string())
                })?;
                Ok(BackendConfig::Disk(disk.clone()))
            }
            "openlist" => {
                let open_list =
                    raw_config.open_list.as_ref().ok_or_else(|| {
                        ConfigError::MissingConfig("OpenList".to_string())
                    })?;
                Ok(BackendConfig::OpenList(open_list.clone()))
            }
            "direct_link" => {
                let direct_link =
                    raw_config.direct_link.as_ref().ok_or_else(|| {
                        ConfigError::MissingConfig("DirectLink".to_string())
                    })?;
                Ok(BackendConfig::DirectLink(direct_link.clone()))
            }
            other => Err(ConfigError::InvalidBackendType(other.to_string())),
        }
    }
}
