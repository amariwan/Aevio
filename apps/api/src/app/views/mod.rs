use std::net::SocketAddr;

use axum::Router;
use axum::routing::{get, post, put};

use crate::app::config::ServerConfig;
use crate::app::core::database;
use crate::app::state::AppState;

mod auth;
mod health;
mod plugin_workspace;
mod plugins;
mod sync;
mod workspaces;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::from_env()?;
    let database = database::DatabaseBackend::open(&config.database).await?;
    auth::seed_admin(&database, &config).await?;
    let state = AppState::new(&config, database);
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(config.bind.parse::<SocketAddr>()?).await?;
    println!("Aevio sync server listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health::health))
        .route("/readyz", get(health::readiness))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/plugins", get(plugins::list))
        .route("/api/v1/plugins/{plugin_id}", get(plugins::detail))
        .route(
            "/api/v1/plugins/{plugin_id}/releases/{version}",
            post(plugins::publish),
        )
        .route(
            "/api/v1/plugins/{plugin_id}/releases/{version}/revoke",
            post(plugins::revoke),
        )
        .route(
            "/api/v1/plugins/{plugin_id}/releases/{version}/artifact",
            get(plugins::download),
        )
        .route(
            "/api/v1/plugins/publishers/{publisher_id}/keys",
            post(plugins::register_publisher_key),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/plugins",
            get(plugin_workspace::list),
        )
        .route(
            "/api/v1/workspaces/{workspace_id}/plugins/{plugin_id}",
            put(plugin_workspace::put),
        )
        .route("/api/v1/workspaces", get(workspaces::list))
        .route(
            "/api/v1/workspaces/{workspace_id}/members",
            post(workspaces::add_member),
        )
        .route("/api/v1/sync/push", post(sync::push))
        .route("/api/v1/sync/pull", get(sync::pull))
        .with_state(state)
}
