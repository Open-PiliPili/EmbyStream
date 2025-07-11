use std::{
    fs,
    path::{Path, PathBuf},
    process,
};

use clap::Parser;
use directories::BaseDirs;
use libc;
use serde::Deserialize;

use super::error::ConfigError;
use crate::config::{
    backend::{Backend, BackendConfig},
    frontend::Frontend,
    general::{General, UserAgent},
    types::RawConfig,
};
use crate::{CONFIG_LOGGER_DOMAIN, cli::Cli, debug_log, error_log, info_log, warn_log};

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
    pub frontend: Frontend,
    pub backend: Backend,
    pub backend_config: BackendConfig,
}

impl Config {
    pub fn load_or_init() -> Result<Self, ConfigError> {
        let cli = Cli::parse();

        if let Some(path) = Self::find_config_path(cli.config)? {
            info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Loading config file at {}",
                path.display()
            );
            return Self::load_from_path(&path);
        }

        let default_path = Self::get_default_config_path()?.join(CONFIG_FILE_NAME);
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

        let backend_config = match raw_config.general.backend_type.as_str() {
            "disk" => BackendConfig::Disk(raw_config.disk.ok_or_else(|| {
                ConfigError::InvalidBackendType(format!("{} backend not configured", "disk"))
            })?),
            "openlist" => BackendConfig::OpenList(raw_config.open_list.ok_or_else(|| {
                ConfigError::InvalidBackendType(format!("{} backend not configured", "openlist"))
            })?),
            "direct_link" => {
                BackendConfig::DirectLink(raw_config.direct_link.ok_or_else(|| {
                    ConfigError::InvalidBackendType(format!(
                        "{} backend not configured",
                        "direct_link"
                    ))
                })?)
            }
            other => return Err(ConfigError::InvalidBackendType(other.to_string())),
        };

        Ok(Config {
            path: path.to_path_buf(),
            general: raw_config.general,
            user_agent: raw_config.user_agent,
            frontend: raw_config.frontend,
            backend: raw_config.backend,
            backend_config,
        })
    }

    fn find_config_path(cli_path: Option<PathBuf>) -> Result<Option<PathBuf>, ConfigError> {
        // Check CLI-provided path first
        if let Some(path) = cli_path {
            if path.exists() {
                debug_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "Found config file at {} from command line arguments",
                    path.display()
                );
                return Ok(Some(path));
            }
            warn_log!(
                CONFIG_LOGGER_DOMAIN,
                "Specified config file at {} does not exist",
                path.display()
            );
        }

        // Check Docker path
        let docker_path = Path::new(DOCKER_CONFIG_PATH);
        if docker_path.exists() {
            debug_log!(
                CONFIG_LOGGER_DOMAIN,
                "Found config file at Docker default location: {}",
                docker_path.display()
            );
            return Ok(Some(docker_path.to_path_buf()));
        }

        // Check default config path
        let default_path = Self::get_default_config_path()?.join(CONFIG_FILE_NAME);
        if default_path.exists() {
            debug_log!(
                CONFIG_LOGGER_DOMAIN,
                "Found config file at default location: {}",
                default_path.display()
            );
            return Ok(Some(default_path));
        }

        Ok(None)
    }

    fn get_default_config_path() -> Result<PathBuf, ConfigError> {
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
