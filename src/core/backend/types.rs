use crate::config::backend::{
    Backend, types::BackendConfig as StreamBackendConfig,
};

#[derive(Clone, Debug)]
pub struct BackendConfig {
    pub crypto_key: String,
    pub crypto_iv: String,
    pub backend: Backend,
    pub backend_config: StreamBackendConfig,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct ContentRange {
    pub(crate) start: u64,
    pub(crate) end: u64,
    pub(crate) total_size: u64,
}

impl ContentRange {
    pub fn length(&self) -> u64 {
        self.end - self.start + 1
    }

    pub fn is_full_range(&self) -> bool {
        self.start == 0 && self.end >= self.total_size.saturating_sub(1)
    }
}

#[derive(Debug)]
pub enum RangeParseError {
    Malformed,
    Unsatisfiable,
}

pub struct ClientInfo {
    pub(crate) id: Option<String>,
    pub(crate) user_agent: Option<String>,
    pub(crate) ip: Option<String>,
}

impl ClientInfo {
    pub fn new(
        id: Option<String>,
        user_agent: Option<String>,
        ip: Option<String>,
    ) -> ClientInfo {
        Self { id, user_agent, ip }
    }
}
