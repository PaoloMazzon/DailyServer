use crate::state::database::DatabaseAccessor;
use crate::util::config::ServerConfig;

/// Meant to be used in an Arc and passed to
#[allow(unused)]
#[derive(Clone)]
pub struct RestState {
    pub config: ServerConfig,
    pub accessor: DatabaseAccessor,
}

impl RestState {
}