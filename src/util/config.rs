use serde::{Deserialize};
use std::fs;
use std::path::Path;
use clap::Parser;
use std::sync::{OnceLock};
use anyhow::anyhow;
use spdlog::prelude::*;
use ignore::gitignore::{Gitignore};
use crate::util::graceful_shutdown::instant_kill_program;

/// Global ignore list for get requests
static IGNORE_LIST: OnceLock<Gitignore> = OnceLock::new();

/// Config that will be loaded at launch
#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub port: u32,
    pub log_filename: String,
    pub ignore_filename: String,
    pub daily_seed_cache: String,
    pub forbidden_filename: String,
    pub not_found_filename: String,
    pub unauthorized_filename: String,
}

impl ServerConfig {
    fn default_config() -> Self {
        ServerConfig {
            port: 3000,
            log_filename: "logs/server.log".to_string(),
            ignore_filename: ".ignore".to_string(),
            daily_seed_cache: "seed_cache".to_string(),
            forbidden_filename: "forbidden.html".to_string(),
            not_found_filename: "not_found.html".to_string(),
            unauthorized_filename: "unauthorized.html".to_string(),
        }
    }

    /// Load config from a path or provide defaults if it doesn't exist there
    pub fn load(path: &Path) -> Self {
        let mut found_config = false;
        let config = match fs::read(path) {
            Ok(file_vec) => {
                found_config = true;
                serde_json::from_slice(&file_vec).unwrap_or(ServerConfig::default_config())
            },
            Err(_) => ServerConfig::default_config()
        };

        match found_config {
            true => info!("Found a config at {:?}\n{:#?}", path, config),
            false => warn!("Failed to find a config at {:?}\n{:#?}", path, config),
        }

        config
    }
}

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CliConfig {
    /// Location of the config file
    #[arg(short, long, default_value_t = String::from("/opt/daily_server/config.json"))]
    pub config_file: String
}

pub fn init_ignore_list(path: &Path) -> Result<(), anyhow::Error> {
    if !path.exists() {
        return Err(anyhow!("Ignore list path {:?} is invalid.", path))
    }
    IGNORE_LIST.get_or_init(|| {
        let (gitignore, errors) = Gitignore::new(path);
        if let Some(e) = errors {
            warn!("Errors parsing ignore file: {}", e);
        }
        gitignore
    });
    Ok(())
}

/// Returns true if this file should be ignored for get requests
pub fn file_in_ignore_list(path: &Path) -> bool {
    match IGNORE_LIST.get() {
        Some(g) => g.matched(path, path.is_dir()).is_ignore(),
        None => {
            error!("Ignore list was not initialized.");
            instant_kill_program();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use super::*;
    use tempfile::NamedTempFile;

    fn fake_ignore_list() -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(".env\n".as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_load_config() {
        ServerConfig::load(Path::new("test.json"));
    }

    #[test]
    fn test_file_in_ignore_list() {
        init_ignore_list(Path::new(fake_ignore_list().path().to_str().unwrap())).unwrap();
        assert!(file_in_ignore_list(Path::new(".env")));
    }
}