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
use tokio::sync::OnceCell;

use super::{
    backend::{Backend, BackendConfig},
    error::ConfigError,
    frontend::Frontend,
    general::{General, StreamMode, UserAgent},
    http2::Http2,
    types::{FallbackConfig, RawConfig},
};

use crate::cli::RunArgs;
use crate::config::backend::types::{
    BackendFallbackConfig, BackendRouteConfig, BackendRoutingConfig,
};
use crate::config::general::{Log, types::Emby};
use crate::core::backend::types::{
    BackendConfig as CoreBackendConfig, BackendRoute, BackendRoutes,
};
use crate::util::resolve_fallback_video_path;
use crate::{
    CONFIG_LOGGER_DOMAIN, config_error_log, config_info_log, config_warn_log,
};

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

        Self::validate_stream_mode(&raw_config)?;
        let (backend_config, backend_routes) =
            Self::build_backend_configs(&raw_config, path)?;

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

    /// Validate stream mode configuration
    fn validate_stream_mode(raw_config: &RawConfig) -> Result<(), ConfigError> {
        let stream_mode = &raw_config.general.stream_mode;

        if (stream_mode == &StreamMode::Frontend
            || stream_mode == &StreamMode::Dual)
            && raw_config.frontend.is_none()
        {
            return Err(ConfigError::MissingConfig("Frontend".to_string()));
        }

        Ok(())
    }

    /// Build backend configurations based on stream mode
    fn build_backend_configs(
        raw_config: &RawConfig,
        path: &Path,
    ) -> Result<(Option<BackendConfig>, Option<BackendRoutes>), ConfigError>
    {
        let stream_mode = &raw_config.general.stream_mode;
        let needs_backend = stream_mode == &StreamMode::Frontend
            || stream_mode == &StreamMode::Backend
            || stream_mode == &StreamMode::Dual;

        if !needs_backend {
            return Ok((None, None));
        }

        if raw_config.backend.is_none() {
            return Err(ConfigError::MissingConfig("Backend".to_string()));
        }

        Self::log_configured_routes(raw_config);
        let routes = Self::build_backend_routes(raw_config, path)?;

        Ok((None, Some(routes)))
    }

    /// Log all configured routes (enabled and disabled)
    fn log_configured_routes(raw_config: &RawConfig) {
        let backend_routes = raw_config
            .backend
            .as_ref()
            .map(|b| b.routes.clone())
            .unwrap_or_default();

        if !backend_routes.is_empty() {
            config_info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Found {} backend route(s) in configuration",
                backend_routes.len()
            );
            for (i, route) in backend_routes.iter().enumerate() {
                let status = if route.enable { "ENABLED" } else { "DISABLED" };
                config_info_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "  Route #{}: [{}] pattern=\"{}\", backend_type=\"{}\"",
                    i + 1,
                    status,
                    route.pattern,
                    route.backend_type
                );
            }
        } else {
            config_warn_log!(
                CONFIG_LOGGER_DOMAIN,
                "DEPRECATED: backend_type in [General] section is deprecated. \
                Please configure [[Backend.Routes]] and [Backend.Fallback] instead. \
                Using backend_type=\"{}\" for default route.",
                raw_config.general.backend_type
            );
        }
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
        let routing_config = Self::get_routing_config(raw_config);

        if !routing_config.enable {
            return Self::build_default_routes(raw_config, config_path);
        }

        let routes = Self::build_route_list(raw_config, config_path)?;
        let fallback_config =
            Self::build_fallback_config(raw_config, config_path)?;

        Ok(BackendRoutes {
            routes,
            fallback: fallback_config,
            match_before_rewrite: routing_config.match_before_rewrite,
            match_priority_first: routing_config.match_priority == "first",
        })
    }

    /// Get routing configuration from raw config
    fn get_routing_config(raw_config: &RawConfig) -> BackendRoutingConfig {
        raw_config
            .backend
            .as_ref()
            .and_then(|b| b.routing.clone())
            .unwrap_or_else(|| BackendRoutingConfig {
                enable: false,
                match_before_rewrite: false,
                match_priority: "first".to_string(),
            })
    }

    /// Build default routes when routing is disabled
    fn build_default_routes(
        raw_config: &RawConfig,
        config_path: &Path,
    ) -> Result<BackendRoutes, ConfigError> {
        let default_route = BackendRouteConfig {
            enable: true,
            pattern: ".*".to_string(),
            backend_type: raw_config.general.backend_type.clone(),
        };

        let backend_config_enum = Self::get_backend_config_by_type(
            &default_route.backend_type,
            raw_config,
        )?;
        let backend_config = Self::build_backend_config_instance(
            raw_config,
            config_path,
            backend_config_enum,
        );

        let fallback_config_enum = Self::get_backend_config_by_type(
            &default_route.backend_type,
            raw_config,
        )?;
        let fallback_config = Self::build_backend_config_instance(
            raw_config,
            config_path,
            fallback_config_enum,
        );

        Ok(BackendRoutes {
            routes: vec![BackendRoute {
                pattern: default_route.pattern,
                regex: OnceCell::new(),
                backend_config,
            }],
            fallback: fallback_config,
            match_before_rewrite: false,
            match_priority_first: true,
        })
    }

    /// Build list of route rules from configuration
    fn build_route_list(
        raw_config: &RawConfig,
        config_path: &Path,
    ) -> Result<Vec<BackendRoute>, ConfigError> {
        let backend_routes = raw_config
            .backend
            .as_ref()
            .map(|b| b.routes.clone())
            .unwrap_or_default();

        let enabled_routes: Vec<_> =
            backend_routes.into_iter().filter(|r| r.enable).collect();

        Self::log_enabled_routes(&enabled_routes, raw_config);

        let routes_to_process = if enabled_routes.is_empty() {
            vec![BackendRouteConfig {
                enable: true,
                pattern: ".*".to_string(),
                backend_type: raw_config.general.backend_type.clone(),
            }]
        } else {
            enabled_routes
        };

        let mut routes = Vec::new();
        for (index, route_config) in routes_to_process.iter().enumerate() {
            Self::validate_route_pattern(&route_config.pattern, index + 1)?;

            let backend_config_enum = Self::get_backend_config_by_type(
                &route_config.backend_type,
                raw_config,
            )?;
            let backend_config = Self::build_backend_config_instance(
                raw_config,
                config_path,
                backend_config_enum,
            );

            routes.push(BackendRoute {
                pattern: route_config.pattern.clone(),
                regex: OnceCell::new(),
                backend_config,
            });
        }

        Ok(routes)
    }

    /// Log enabled routes information
    fn log_enabled_routes(
        enabled_routes: &[BackendRouteConfig],
        raw_config: &RawConfig,
    ) {
        if !enabled_routes.is_empty() {
            config_info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Using {} enabled route(s) for routing",
                enabled_routes.len()
            );
        } else if !raw_config
            .backend
            .as_ref()
            .map(|b| b.routes.is_empty())
            .unwrap_or(true)
        {
            config_info_log!(
                CONFIG_LOGGER_DOMAIN,
                "No enabled routes found, using default route with backend_type=\"{}\"",
                raw_config.general.backend_type
            );
        }
    }

    /// Validate regex pattern for a route
    fn validate_route_pattern(
        pattern: &str,
        index: usize,
    ) -> Result<(), ConfigError> {
        Regex::new(pattern).map_err(|e| {
            config_error_log!(
                CONFIG_LOGGER_DOMAIN,
                "Invalid regex pattern in route #{}: \"{}\", error: {}",
                index,
                pattern,
                e
            );
            ConfigError::InvalidRegex {
                pattern: pattern.to_string(),
                error: e.to_string(),
            }
        })?;
        Ok(())
    }

    /// Build fallback backend configuration
    fn build_fallback_config(
        raw_config: &RawConfig,
        config_path: &Path,
    ) -> Result<CoreBackendConfig, ConfigError> {
        let backend_fallback =
            raw_config.backend.as_ref().and_then(|b| b.fallback.clone());

        let fallback_config_enum = if let Some(fallback) = &backend_fallback {
            Self::resolve_fallback_backend_type(fallback, raw_config)?
        } else {
            Self::resolve_legacy_fallback_backend_type(raw_config)?
        };

        Ok(Self::build_backend_config_instance(
            raw_config,
            config_path,
            fallback_config_enum,
        ))
    }

    /// Resolve fallback backend type from configured fallback
    fn resolve_fallback_backend_type(
        fallback: &BackendFallbackConfig,
        raw_config: &RawConfig,
    ) -> Result<BackendConfig, ConfigError> {
        if fallback.enable {
            config_info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Fallback backend enabled: type=\"{}\"",
                fallback.backend_type
            );
            Self::get_backend_config_by_type(&fallback.backend_type, raw_config)
        } else {
            let backend_type = raw_config.general.backend_type.as_str();
            config_info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Fallback backend disabled, using backend_type=\"{}\" as fallback",
                backend_type
            );
            config_warn_log!(
                CONFIG_LOGGER_DOMAIN,
                "DEPRECATED: backend_type in [General] section is deprecated. \
                Please configure [Backend.Fallback] with enable=true instead."
            );
            Self::get_backend_config_by_type(backend_type, raw_config)
        }
    }

    /// Resolve fallback backend type from legacy backend_type
    fn resolve_legacy_fallback_backend_type(
        raw_config: &RawConfig,
    ) -> Result<BackendConfig, ConfigError> {
        let backend_type = raw_config.general.backend_type.as_str();
        config_info_log!(
            CONFIG_LOGGER_DOMAIN,
            "No [Backend.Fallback] configured, using backend_type=\"{}\" as fallback",
            backend_type
        );
        config_warn_log!(
            CONFIG_LOGGER_DOMAIN,
            "DEPRECATED: backend_type in [General] section is deprecated. \
            Please configure [Backend.Fallback] with enable=true instead."
        );
        Self::get_backend_config_by_type(backend_type, raw_config)
    }

    /// Build a BackendConfig instance from raw config and backend config enum
    fn build_backend_config_instance(
        raw_config: &RawConfig,
        config_path: &Path,
        backend_config_enum: BackendConfig,
    ) -> CoreBackendConfig {
        let backend =
            raw_config.backend.as_ref().expect("Backend config missing");
        let fallback_video_path = resolve_fallback_video_path(
            &raw_config.fallback.video_missing_path,
            config_path,
        );

        CoreBackendConfig {
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
            "disk" => raw_config
                .disk
                .as_ref()
                .map(|d| BackendConfig::Disk(d.clone()))
                .ok_or_else(|| ConfigError::MissingConfig("Disk".to_string())),
            "openlist" => raw_config
                .open_list
                .as_ref()
                .map(|o| BackendConfig::OpenList(o.clone()))
                .ok_or_else(|| {
                    ConfigError::MissingConfig("OpenList".to_string())
                }),
            "direct_link" => raw_config
                .direct_link
                .as_ref()
                .map(|d| BackendConfig::DirectLink(d.clone()))
                .ok_or_else(|| {
                    ConfigError::MissingConfig("DirectLink".to_string())
                }),
            other => Err(ConfigError::InvalidBackendType(other.to_string())),
        }
    }
}
