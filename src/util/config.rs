use serde::{Deserialize};
use std::fs;
use std::path::Path;
use clap::Parser;

/// Config that will be loaded at launch
#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub port: u32,
    pub log_filename: String
}

impl ServerConfig {
    fn default_config() -> Self {
        ServerConfig {
            port: 3000,
            log_filename: "logs/server.log".to_string(),
        }
    }

    /// Load config from a path or provide defaults if it doesn't exist there
    pub fn load(path: &Path) -> Self {
        match fs::read(path) {
            Ok(file_vec) => {
                serde_json::from_slice(&file_vec).unwrap_or(ServerConfig::default_config())
            },
            Err(_) => ServerConfig::default_config()
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct CliConfig {
    /// Location of the config file
    #[arg(short, long, default_value_t = String::from("config.json"))]
    pub config_file: String
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_config() {
        ServerConfig::load(Path::new("test.json"));
    }
}