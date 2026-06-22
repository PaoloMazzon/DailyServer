use std::sync::Arc;
use axum::{Router, routing::*};
use spdlog::{sink::RotatingFileSink, sink::StdStreamSink, sink::RotationPolicy, prelude::*};
use std::path::Path;
use clap::Parser;
mod endpoints;
mod util;
mod state;
use crate::endpoints::api;
use crate::util::config::{ServerConfig, CliConfig};

fn setup_logging(config: &ServerConfig) -> Result<(), String> {
    let log_path = config.log_filename.as_str();
    let max_files = 30;
    let policy = RotationPolicy::Daily { hour: 0, minute: 0 };

    let rotating_sink = RotatingFileSink::builder()
        .base_path(log_path)
        .rotation_policy(policy)
        .max_files(max_files)
        .rotate_on_open(false)
        .build_arc().map_err(|e| format!("Failed to create rotating file sink: {}", e))?;

    let stdout_sink = StdStreamSink::builder()
        .stdout()
        .via_print_macro() 
        .build_arc().map_err(|e| format!("Failed to create stdout sink: {}", e))?;

    let logger = Logger::builder()
        .name("server_logger")
        .sink(rotating_sink)
        .sink(stdout_sink)
        .flush_level_filter(LevelFilter::All)
        .build().map_err(|e| format!("Failed to create logger: {}", e))?;

    // 5. Register it globally
    spdlog::set_default_logger(Arc::new(logger));

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), String> {
    // Load configs and setup logging
    let args = CliConfig::parse();
    let config = ServerConfig::load(Path::new(&args.config_file));
    setup_logging(&config)?;

    // Bind endpoints
    let app = Router::new()
        .route("/api", get(api::api_endpoint_get))
        .route("/api", post(api::api_endpoint_post));

    // Start server
    let ip = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&ip).await.unwrap();
    info!("Server started successfully at {}", ip);
    axum::serve(listener, app).await.unwrap();
    Ok(())
}
