use crate::config::AppConfig;
use crate::daemon::DaemonClient;
use crate::events::{self, EventHub};
use crate::routes;
use crate::storage::ProfileStore;
use axum::Router;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub daemon: DaemonClient,
    pub profiles: ProfileStore,
    pub events: EventHub,
}

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    let daemon = DaemonClient::new(config.socket_path.clone());
    let profiles = ProfileStore::new(config.profile_store_path.clone());
    let events = EventHub::new();
    events::start_polling(daemon.clone(), events.clone());
    let state = AppState {
        config,
        daemon,
        profiles,
        events,
    };

    let app: Router = routes::router(state.clone());
    let addr = state.config.bind_addr();

    tracing::info!(
        "Backend listening on {addr}, profile store at {}",
        state.profiles.path().display()
    );
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
