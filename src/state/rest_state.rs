use crate::util::config::ServerConfig;

/// Meant to be used in an Arc and passed to
#[allow(unused)]
#[derive(Clone, Debug)]
pub struct RestState {
    pub config: ServerConfig,
}

impl RestState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config
        }
    }
}