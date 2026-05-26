mod budget;
mod index;
mod routes;
mod state;
mod targeting;

use std::sync::Arc;
use axum::{Router, routing::{get, post}};
use common::Config;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing::info;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    common::telemetry::init(&cfg.log_level);

    let pg = PgPoolOptions::new()
        .max_connections(10)
        .connect(&cfg.database_url)
        .await?;

    let redis = redis::Client::open(cfg.redis_url.as_str())?;
    let redis_mgr = redis::aio::ConnectionManager::new(redis).await?;

    let state = Arc::new(AppState::new(pg, redis_mgr, cfg.clone()));

    // Seed the in-memory campaign index and start background refresh
    state.index.refresh(&state.pg).await?;
    index::start_refresh_task(Arc::clone(&state));

    let app = Router::new()
        .route("/bid", post(routes::bid::handle))
        .route("/win", get(routes::win::handle))
        .route("/imp/:token", get(routes::imp::handle))
        .route("/click/:token", get(routes::click::handle))
        .route("/health", get(routes::health::handle))
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::clone(&state));

    let addr = cfg.bidder_addr();
    info!("bidder listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
