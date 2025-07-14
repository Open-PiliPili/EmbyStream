use std::env;

use crate::system::Environment;

#[derive(Debug)]
pub struct SystemInfo {
    pub version: String,
    pub environment: Environment,
}

impl SystemInfo {
    pub fn new() -> Self {
        SystemInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: Self::detect_environment(),
        }
    }

    fn detect_environment() -> Environment {
        if std::fs::metadata("/.dockerenv").is_ok() {
            return Environment::Docker;
        }

        // Check OS via std::env
        match env::consts::OS {
            "linux" => Environment::Linux,
            "macos" => Environment::MacOS,
            "windows" => Environment::Windows,
            _ => Environment::Unknown,
        }
    }

    pub fn get_version(&self) -> &str {
        &self.version
    }

    pub fn get_environment(&self) -> &Environment {
        &self.environment
    }

    pub fn get_user_agent(&self) -> String {
        format!("PStreamer/{}", self.version)
    }
}
