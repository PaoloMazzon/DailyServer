extern crate core;

use std::sync::{Arc};
use axum::{Router, routing::*};
use spdlog::{sink::RotatingFileSink, sink::StdStreamSink, sink::RotationPolicy, prelude::*};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use clap::Parser;

mod endpoints;
mod util;
mod state;
use crate::endpoints::{api, general_get};
use crate::state::database::Database;
use crate::state::rest_state::RestState;
use crate::util::config::{ServerConfig, CliConfig, init_ignore_list};
use crate::util::daily_seed::init_daily_seed_task;
use crate::util::graceful_shutdown::kill_program;

fn setup_logging(log_filename: String) -> anyhow::Result<()> {
    let log_path = log_filename.as_str();
    let max_files = 30;
    let policy = RotationPolicy::Daily { hour: 0, minute: 0 };
    let filter_policy = match cfg!(debug_assertions) {
        true => LevelFilter::MoreSevereEqual(Level::Debug),
        false => LevelFilter::MoreSevereEqual(Level::Info)
    };

    let rotating_sink = RotatingFileSink::builder()
        .base_path(log_path)
        .rotation_policy(policy)
        .max_files(max_files)
        .rotate_on_open(false)
        .build_arc()?;

    let stdout_sink = StdStreamSink::builder()
        .stdout()
        .via_print_macro() 
        .build_arc()?;

    let logger = Logger::builder()
        .name("server_logger")
        .sink(rotating_sink)
        .sink(stdout_sink)
        .level_filter(filter_policy)
        .flush_level_filter(LevelFilter::All)
        .build()?;

    // 5. Register it globally
    spdlog::set_default_logger(Arc::new(logger));

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    info!("Starting daily server version v{}.{}", env!("CARGO_PKG_VERSION"), option_env!("GITHUB_RUN_NUMBER").unwrap_or("dev"));

    // Setup ctrl+c functionality
    let ctrl_c_kill_signal = Arc::new(AtomicBool::new(false));
    let listener_kill_signal = ctrl_c_kill_signal.clone();
    let server_kill = ctrl_c_kill_signal.clone();
    ctrlc::set_handler(move || {
        ctrl_c_kill_signal.store(true, Ordering::Relaxed);
    })?;
    tokio::task::spawn(async move {
        while !listener_kill_signal.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        kill_program();
    });
    let server_shutdown_future = async move {
        while !server_kill.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };

    // Load configs and setup logging
    let args = CliConfig::parse();
    debug!("Parsed CLI options.");
    
    let config = ServerConfig::load(Path::new(&args.config_file));
    debug!("Loaded config.");
    
    setup_logging(config.log_filename.clone())?;
    debug!("Setup logging.");
    
    init_ignore_list(Path::new(config.ignore_filename.as_str()))?;
    debug!("Created ignore list.");
    
    init_daily_seed_task(&config).await?;
    debug!("Initialized daily seed task.");
    
    let database = Database::open(config.highscore_db.as_str()).await?;
    debug!("Initialized database.");

    // Bind endpoints
    let app = Router::new()
        .route("/api{*wildcard}", post(api::api_endpoint_post))
        .route("/api", post(api::api_endpoint_post))
        .route("/api{*wildcard}", get(api::api_endpoint_get))
        .route("/api", get(api::api_endpoint_get))
        .route("/{*wildcard}", get(general_get::endpoint_get))
        .route("/", get(general_get::endpoint_get))
        .with_state(RestState { config: config.clone(), accessor: database.get_accessor().await });

    // Start server
    let ip = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&ip).await?;
    info!("Server started successfully at {}", ip);
    axum::serve(listener, app)
        .with_graceful_shutdown(server_shutdown_future)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::setup_logging;

    #[test]
    fn test_logger() {
        setup_logging(String::from("/tmp/asd.log")).unwrap();
    }
}
