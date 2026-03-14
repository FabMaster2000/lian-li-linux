use axum::middleware;
use axum::routing::{get, post, put};
use axum::Router;

use crate::app::AppState;
use crate::auth;
use crate::handlers;

pub fn router(state: AppState) -> Router {
    let public = Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/version", get(handlers::version));

    let protected = Router::new()
        .route("/api/runtime", get(handlers::runtime))
        .route("/api/ws", get(handlers::websocket_events))
        .route(
            "/api/config",
            get(handlers::get_api_config).post(handlers::set_api_config),
        )
        .route("/api/profiles", get(handlers::list_profiles).post(handlers::create_profile))
        .route(
            "/api/profiles/:id/apply",
            post(handlers::apply_profile),
        )
        .route(
            "/api/profiles/:id",
            put(handlers::update_profile).delete(handlers::delete_profile),
        )
        .route("/api/daemon/status", get(handlers::daemon_status))
        .route("/api/devices", get(handlers::list_devices))
        .route("/api/devices/:id", get(handlers::get_device))
        .route(
            "/api/devices/:id/lighting",
            get(handlers::get_lighting_state),
        )
        .route(
            "/api/devices/:id/lighting/color",
            post(handlers::set_lighting_color),
        )
        .route(
            "/api/devices/:id/lighting/effect",
            post(handlers::set_lighting_effect),
        )
        .route(
            "/api/devices/:id/lighting/brightness",
            post(handlers::set_lighting_brightness),
        )
        .route("/api/devices/:id/fans", get(handlers::get_fan_state))
        .route(
            "/api/devices/:id/fans/manual",
            post(handlers::set_fan_manual),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    public.merge(protected).with_state(state)
}
