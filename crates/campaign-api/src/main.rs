mod models;
mod routes;
mod state;

use std::sync::Arc;
use axum::{Router, routing::{get, post}};
use common::Config;
use sqlx::postgres::PgPoolOptions;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    common::telemetry::init(&cfg.log_level);

    let pg = PgPoolOptions::new()
        .max_connections(20)
        .connect(&cfg.database_url)
        .await?;

    sqlx::migrate!("../../migrations").run(&pg).await?;

    let state = Arc::new(AppState { pg });

    let app = Router::new()
        // Advertisers
        .route("/advertisers",           post(routes::advertisers::create))
        .route("/advertisers/:id",       get(routes::advertisers::get_one))
        .route("/advertisers/:id/topup", post(routes::advertisers::topup))
        // Campaigns
        .route("/campaigns",             get(routes::campaigns::list).post(routes::campaigns::create))
        .route("/campaigns/:id",         get(routes::campaigns::get_one).put(routes::campaigns::update))
        .route("/campaigns/:id/pause",   post(routes::campaigns::pause))
        .route("/campaigns/:id/activate",post(routes::campaigns::activate))
        // Targeting
        .route("/campaigns/:id/targeting", get(routes::targeting::get).put(routes::targeting::update))
        // Creatives
        .route("/campaigns/:id/creatives", post(routes::creatives::create))
        .route("/creatives/:id/approve",   post(routes::creatives::approve))
        .route("/creatives/:id/reject",    post(routes::creatives::reject))
        // Reporting
        .route("/campaigns/:id/report",    get(routes::reporting::campaign_report))
        // Health
        .route("/health", get(routes::health::handle))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = cfg.api_addr();
    info!("campaign api listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
