// Publisher-tag service — serves the JS snippet publishers paste on their sites
// and handles ad requests from those sites directly (bypassing exchanges).
// This gives us 100% margin on known GCC publishers.

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use common::Config;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing::info;

struct AppState {
    pg: sqlx::PgPool,
    cfg: Config,
}

#[derive(Serialize)]
struct AdResponse {
    ad_markup: String,
    width: i32,
    height: i32,
    click_url: String,
    imp_url: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::load()?;
    common::telemetry::init(&cfg.log_level);

    let pg = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database_url)
        .await?;

    let state = Arc::new(AppState { pg, cfg });

    let app = Router::new()
        .route("/tag.js",         get(serve_tag_js))
        .route("/serve/:tag_id",  get(serve_ad))
        .route("/health",         get(|| async { StatusCode::OK }))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:8082";
    info!("publisher-tag service listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// The JS snippet publishers paste on their sites.
/// Fetches an ad from /serve/:tag_id and injects it into the page.
async fn serve_tag_js(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let hostname = &state.cfg.public_hostname;
    let js = format!(
        r#"(function(){{
  var s=document.currentScript||document.scripts[document.scripts.length-1];
  var tagId=s.getAttribute('data-tag');
  if(!tagId)return;
  fetch('https://{hostname}/tag/serve/'+tagId)
    .then(function(r){{return r.json()}})
    .then(function(ad){{
      var div=document.createElement('div');
      div.innerHTML=ad.ad_markup;
      s.parentNode.insertBefore(div,s.nextSibling);
      new Image().src=ad.imp_url;
    }})
    .catch(function(){{}});
}})();
"#
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8"),
         (header::CACHE_CONTROL, "public, max-age=300")],
        js,
    )
}

/// Serves a direct ad to a publisher who has embedded our tag.
/// Looks up the tag_id → publisher → selects a matching campaign via the bidder API.
/// For Phase 1 this returns a placeholder; Phase 2 wires it to the bidder engine.
async fn serve_ad(
    State(_state): State<Arc<AppState>>,
    Path(tag_id): Path<String>,
) -> impl IntoResponse {
    info!(tag_id = %tag_id, "direct ad request");

    // Phase 2: look up publisher, call internal bidder, return winning ad.
    // For now return a no-fill response so publishers can integrate without blocking.
    (StatusCode::NO_CONTENT).into_response()
}
