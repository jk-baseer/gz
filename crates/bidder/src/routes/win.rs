use std::sync::Arc;
use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

use crate::{budget, state::AppState};

#[derive(Deserialize)]
pub struct WinParams {
    /// Auction ID from the original bid request
    aid: String,
    /// Our bid ID
    bid: String,
    /// Campaign ID
    cid: Uuid,
    /// Clearing price — the ${AUCTION_PRICE} macro filled by the exchange (USD CPM)
    price: f64,
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Query(params): Query<WinParams>,
) -> StatusCode {
    info!(
        auction_id = %params.aid,
        bid_id = %params.bid,
        campaign_id = %params.cid,
        clearing_price = params.price,
        "win notice received"
    );

    let clearing_cents = (params.price * 100.0).round() as i64;

    // Find the bid price we originally reserved so we can correct the delta
    let campaigns = state.index.get_all().await;
    let bid_cents = campaigns
        .iter()
        .find(|c| c.id == params.cid)
        .map(|c| c.bid_price_cpm_cents)
        .unwrap_or(clearing_cents);

    let mut redis = state.redis.clone();
    if let Err(e) = budget::record_win(&mut redis, params.cid, clearing_cents, bid_cents).await {
        warn!(error = %e, "failed to record win in Redis");
    }

    // TODO: publish impression event to Redpanda for ClickHouse ingestion

    StatusCode::OK
}
