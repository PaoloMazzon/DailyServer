use std::sync::Arc;
use axum::{Router, routing::*};
use spdlog::{sink::RotatingFileSink, sink::StdStreamSink, sink::RotationPolicy, prelude::*};
use std::path::Path;
use clap::Parser;
mod endpoints;
mod util;
mod state;
use crate::endpoints::{api, general_get};
use crate::util::config::{ServerConfig, CliConfig, init_ignore_list};
use crate::util::daily_seed::init_daily_seed_task;

fn setup_logging(config: &ServerConfig) -> anyhow::Result<()> {
    let log_path = config.log_filename.as_str();
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
    // Load configs and setup logging
    let args = CliConfig::parse();
    let config = ServerConfig::load(Path::new(&args.config_file));
    setup_logging(&config)?;
    init_ignore_list(Path::new(config.ignore_filename.as_str()))?;
    init_daily_seed_task(&config).await?;

    // Bind endpoints
    let app = Router::new()
        .route("/{*wildcard}", get(general_get::endpoint_get))
        .route("/", get(general_get::endpoint_get))
        .route("/api", post(api::api_endpoint_post));

    // Start server
    let ip = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&ip).await?;
    info!("Server started successfully at {}", ip);
    axum::serve(listener, app).await?;
    Ok(())
}
