use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use directories::BaseDirs;
use libc;
use serde::Deserialize;

use super::error::ConfigError;
use crate::config::{
    backend::{Backend, BackendConfig},
    frontend::Frontend,
    general::{General, StreamMode, UserAgent},
    types::RawConfig,
};
use crate::{CONFIG_LOGGER_DOMAIN, error_log, info_log};

const CONFIG_DIR_NAME: &str = "embystream";
const CONFIG_FILE_NAME: &str = "config.toml";
const DOCKER_CONFIG_PATH: &str = "/config/embystream/config.toml";
const TEMPLATE_CONFIG_PATH: &str = "src/config.toml.template";
const ROOT_CONFIG_PATH: &str = "/root/.config/embystream";

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub path: PathBuf,
    pub general: General,
    pub user_agent: UserAgent,
    pub frontend: Option<Frontend>,
    pub backend: Option<Backend>,
    pub backend_config: Option<BackendConfig>,
}

impl Config {
    pub fn load_or_init(config_path: Option<PathBuf>) -> Result<Self, ConfigError> {
        if let Some(path) = config_path {
            return Self::load_from_path(&path);
        }

        let default_path = Self::get_default_config_path()?.join(CONFIG_FILE_NAME);
        if default_path.exists() {
            info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Loading config file from default location: {}",
                default_path.display()
            );
            return Self::load_from_path(&default_path);
        }

        Self::handle_missing_config(&default_path)?;
        unreachable!();
    }

    pub fn reload(&mut self) -> Result<(), ConfigError> {
        info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Reloading config file at {}",
            self.path.display()
        );
        let new_config = Self::load_from_path(&self.path)?;

        *self = new_config;

        info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Successfully reloaded config file at {}",
            self.path.display()
        );
        Ok(())
    }

    fn load_from_path(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path)?;
        let raw_config: RawConfig = toml::from_str(&content)?;

        let stream_mode = &raw_config.general.stream_mode;

        if (stream_mode == &StreamMode::Frontend || stream_mode == &StreamMode::Dual)
            && raw_config.frontend.is_none()
        {
            return Err(ConfigError::MissingConfig("Frontend".to_string()));
        }

        let backend_config;

        if stream_mode == &StreamMode::Frontend
            || stream_mode == &StreamMode::Backend
            || stream_mode == &StreamMode::Dual
        {
            if raw_config.backend.is_none() {
                return Err(ConfigError::MissingConfig("Backend".to_string()));
            }

            let backend_type = raw_config.general.backend_type.as_str();
            let config = match backend_type.to_lowercase().as_str() {
                "disk" => BackendConfig::Disk(
                    raw_config
                        .disk
                        .ok_or_else(|| ConfigError::MissingConfig("Disk".to_string()))?,
                ),
                "openlist" => BackendConfig::OpenList(
                    raw_config
                        .open_list
                        .ok_or_else(|| ConfigError::MissingConfig("OpenList".to_string()))?,
                ),
                "direct_link" => BackendConfig::DirectLink(
                    raw_config
                        .direct_link
                        .ok_or_else(|| ConfigError::MissingConfig("DirectLink".to_string()))?,
                ),
                other => return Err(ConfigError::InvalidBackendType(other.to_string())),
            };
            backend_config = Some(config);
        } else {
            backend_config = None;
        }

        Ok(Config {
            path: path.to_path_buf(),
            general: raw_config.general,
            user_agent: raw_config.user_agent,
            frontend: raw_config.frontend,
            backend: raw_config.backend,
            backend_config,
        })
    }

    fn get_default_config_path() -> Result<PathBuf, ConfigError> {
        if Path::new(DOCKER_CONFIG_PATH).exists() {
            return Ok(PathBuf::from(DOCKER_CONFIG_PATH));
        }

        let base_dirs = BaseDirs::new().ok_or(ConfigError::NoHomeDir)?;

        let path = if cfg!(target_os = "linux") && unsafe { libc::getuid() } == 0 {
            PathBuf::from(ROOT_CONFIG_PATH)
        } else if cfg!(target_os = "windows") {
            base_dirs.config_dir().join(CONFIG_DIR_NAME)
        } else {
            // macOS and other Unix-like systems
            base_dirs.home_dir().join(".config").join(CONFIG_DIR_NAME)
        };

        Ok(path)
    }

    fn handle_missing_config(target_path: &Path) -> Result<(), ConfigError> {
        let template_path = Path::new(TEMPLATE_CONFIG_PATH);

        if !template_path.exists() {
            error_log!(
                CONFIG_LOGGER_DOMAIN,
                "Missing template config file at {}",
                template_path.display()
            );
            return Err(ConfigError::CopyTemplate(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Template file not found at {}", template_path.display()),
            )));
        }

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::CreateDir {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        fs::copy(template_path, target_path).map_err(ConfigError::CopyTemplate)?;

        info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Created new config file at {} from template",
            target_path.display()
        );

        error_log!(
            CONFIG_LOGGER_DOMAIN,
            "Please configure the new file and restart the application"
        );
        process::exit(0);
    }
}
