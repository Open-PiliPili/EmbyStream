use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    fs,
    path::Path,
};

use serde::Deserialize;
use toml;
use uuid::Uuid;

use crate::config::{
    backend::{BackendConfig, r#type::BackendType},
    frontened::FrontendConfig,
    general::GeneralConfig,
};
use crate::{CONFIG_LOGGER_DOMAIN, Error, error_log, info_log};

/// Top-level configuration structure.
#[derive(Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(rename = "General")]
    general: GeneralConfig,
    #[serde(rename = "Frontend")]
    frontend: Option<FrontendConfig>,
    #[serde(flatten)]
    backend: Option<BackendConfig>,
}

impl Config {
    /// Checks if the provided TOML content has a valid format.
    ///
    /// # Arguments
    ///
    /// * `content` - The TOML content to check.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the TOML format is valid.
    /// * `Err(Error)` if the TOML is invalid.
    fn check_toml_format(content: &str) -> Result<(), Error> {
        // Attempt to parse TOML
        let _: Config = toml::from_str(content).map_err(|e| {
            error_log!(CONFIG_LOGGER_DOMAIN, "Invalid TOML format: {}", e);
            Error::TomlParseError(e)
        })?;

        info_log!(CONFIG_LOGGER_DOMAIN, "TOML format is valid!");
        Ok(())
    }

    /// Loads configuration from the specified file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the config.toml file.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the configuration is loaded successfully.
    /// * `Err(Error)` if the file cannot be read, parsed, or validated.
    pub fn load_from_file(path: &str) -> Result<(), Error> {
        info_log!(CONFIG_LOGGER_DOMAIN, "Loading config from file: {}", path);

        // Read config file
        let content = fs::read_to_string(path).map_err(|e| {
            error_log!(CONFIG_LOGGER_DOMAIN, "Error reading config file: {}", e);
            Error::IoError(e)
        })?;

        // Check TOML format
        Self::check_toml_format(&content)?;

        // Parse config
        let mut config: Config = toml::from_str(&content).map_err(|e| {
            error_log!(CONFIG_LOGGER_DOMAIN, "Error parsing config file: {}", e);
            Error::TomlParseError(e)
        })?;

        // Validate encipher_key length
        let encipher_key_len = config.general.encipher_key.len();
        if encipher_key_len < 6 {
            error_log!(
                CONFIG_LOGGER_DOMAIN,
                "Encipher key must be at least 6 bytes, got {} bytes",
                encipher_key_len
            );
            return Err(Error::InvalidEncipherKey(encipher_key_len));
        }

        // Validate backend configuration
        match (&config.backend, &config.general.backend_type) {
            (Some(BackendConfig::Disk(_)), BackendType::Disk)
            | (Some(BackendConfig::OpenList(_)), BackendType::OpenList)
            | (Some(BackendConfig::DirectLink(_)), BackendType::DirectLink) => {
                // Valid configuration
            }
            (None, _) => {
                error_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "No backend configuration provided for backend type: {:?}",
                    config.general.backend_type
                );
                return Err(Error::InvalidBackendConfig(format!(
                    "{:?}",
                    config.general.backend_type
                )));
            }
            (Some(_), _) => {
                error_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "Backend configuration does not match backend type: {:?}",
                    config.general.backend_type
                );
                return Err(Error::InvalidBackendConfig(format!(
                    "{:?}",
                    config.general.backend_type
                )));
            }
        }

        // Generate and save api_key if empty
        if config.general.api_key.is_empty() {
            info_log!(
                CONFIG_LOGGER_DOMAIN,
                "No api_key found, generating new UUID"
            );
            config.save_api_key(path)?;
        }

        info_log!(CONFIG_LOGGER_DOMAIN, "Configuration loaded successfully");
        Ok(())
    }

    /// Saves a new api_key to the config file and updates the in-memory config.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the config.toml file.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the api_key is saved successfully.
    /// * `Err(Error)` if the file cannot be read or written.
    fn save_api_key(&mut self, path: &str) -> Result<(), Error> {
        info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Saving new api_key to config file: {}",
            path
        );

        // Generate new api_key
        let new_api_key = Uuid::new_v4().to_string();
        self.general.api_key = new_api_key.clone();

        // Read existing config content
        let content = fs::read_to_string(path).map_err(|e| {
            error_log!(
                CONFIG_LOGGER_DOMAIN,
                "Failed to read config file for saving api_key: {}",
                e
            );
            Error::IoError(e)
        })?;

        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let mut found = false;

        // Update api_key line
        for line in lines.iter_mut() {
            if line.trim().starts_with("api_key") {
                *line = format!("api_key = \"{}\"", new_api_key);
                found = true;
                break;
            }
        }

        // Append api_key to [General] section if not found
        if !found {
            let general_index = lines.iter().position(|line| line.trim() == "[General]");
            if let Some(index) = general_index {
                lines.insert(index + 1, format!("api_key = \"{}\"", new_api_key));
            } else {
                error_log!(
                    CONFIG_LOGGER_DOMAIN,
                    "No [General] section found in config file"
                );
                return Err(Error::MissingGeneralSection);
            }
        }

        // Write back to file
        fs::write(path, lines.join("\n")).map_err(|e| {
            error_log!(CONFIG_LOGGER_DOMAIN, "Failed to write config file: {}", e);
            Error::IoError(e)
        })?;

        info_log!(CONFIG_LOGGER_DOMAIN, "api_key saved successfully");
        Ok(())
    }

    /// Initializes the target config file from a template if it does not exist.
    ///
    /// If the target path already exists, no action is taken. Otherwise, the template
    /// file is copied to the target path after validating its TOML format.
    ///
    /// # Arguments
    ///
    /// * `template_path` - Path to the template config.toml file.
    /// * `target_path` - Path where the config.toml file should be created.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the operation is successful or the target file already exists.
    /// * `Err(Error)` if the template cannot be read, is invalid, or the target cannot be written.
    pub fn init_from_template(template_path: &str, target_path: &str) -> Result<(), Error> {
        info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Initializing config from template: {} to {}",
            template_path,
            target_path
        );

        // Check if target path exists
        if Path::new(target_path).exists() {
            info_log!(
                CONFIG_LOGGER_DOMAIN,
                "Target config file already exists: {}",
                target_path
            );
            return Ok(());
        }

        // Read template file
        let content = fs::read_to_string(template_path).map_err(|e| {
            error_log!(CONFIG_LOGGER_DOMAIN, "Failed to read template file: {}", e);
            Error::IoError(e)
        })?;

        // Check TOML format of template
        Self::check_toml_format(&content)?;

        // Copy template to target
        fs::copy(template_path, target_path).map_err(|e| {
            error_log!(
                CONFIG_LOGGER_DOMAIN,
                "Failed to copy template to target: {}",
                e
            );
            Error::IoError(e)
        })?;

        info_log!(
            CONFIG_LOGGER_DOMAIN,
            "Config template copied successfully to {}",
            target_path
        );
        Ok(())
    }

    /// Gets the General configuration.
    pub fn general(&self) -> &GeneralConfig {
        &self.general
    }

    /// Gets the Frontend configuration.
    pub fn frontend(&self) -> Option<&FrontendConfig> {
        self.frontend.as_ref()
    }

    /// Gets the backend configuration, if present and matches the backend type.
    pub fn backend(&self) -> Option<&BackendConfig> {
        self.backend.as_ref()
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "Config {{ general: {}, frontend: {}, backend: {} }}",
            self.general,
            self.frontend
                .as_ref()
                .map_or("None".to_string(), |b| b.to_string()),
            self.backend
                .as_ref()
                .map_or("None".to_string(), |b| b.to_string())
        )
    }
}
