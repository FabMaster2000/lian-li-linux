mod app;
mod auth;
mod config;
mod daemon;
mod errors;
mod events;
mod handlers;
mod models;
mod routes;
mod storage;
#[cfg(test)]
mod tests;

use crate::config::AppConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::load()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(config.log_level.clone()))
        .init();

    app::run(config).await
}
