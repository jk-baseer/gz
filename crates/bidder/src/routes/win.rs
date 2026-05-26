use std::sync::Arc;
use axum::{
    extract::{Query, State},
    http::StatusCode,
};
use redis::AsyncCommands;
use serde::Deserialize;
use tracing::{info, warn};
use uuid::Uuid;

use bidder::{budget, token};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct WinParams {
    aid: String,
    bid: String,
    cid: Uuid,
    exchange: String,
    /// Clearing price in USD CPM — filled by the exchange's ${AUCTION_PRICE} macro
    price: f64,
}

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Query(p): Query<WinParams>,
) -> StatusCode {
    info!(
        auction_id = %p.aid,
        bid_id     = %p.bid,
        campaign   = %p.cid,
        exchange   = %p.exchange,
        clearing   = p.price,
        "win notice"
    );

    let clearing_cents = (p.price * 100.0).round() as i64;
    let mut redis = state.redis.clone();

    // Retrieve + delete the pending impression stored at bid time
    let redis_key = format!("pending_imp:{}", p.bid);
    let json: Option<String> = match redis.get_del(&redis_key).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "failed to retrieve pending impression");
            None
        }
    };

    let pending: Option<token::PendingImpression> = json
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok());

    // Adjust Redis spend: we reserved bid_price at auction; pay only clearing_price
    let bid_cents = pending
        .as_ref()
        .map(|pi| pi.bid_price_cents)
        .unwrap_or(clearing_cents);

    if let Err(e) = budget::record_win(&mut redis, p.cid, clearing_cents, bid_cents).await {
        warn!(error = %e, "failed to record win spend in Redis");
    }

    if let Some(pi) = &pending {
        // Write impression event
        let result = sqlx::query(
            r#"
            INSERT INTO impression_events (
                campaign_id, creative_id, exchange, auction_id,
                bid_price_cents, clearing_price_cents,
                geo_country, device_type, os, site_domain
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
            "#,
        )
        .bind(pi.campaign_id)
        .bind(pi.creative_id)
        .bind(&pi.exchange)
        .bind(&p.aid)
        .bind(pi.bid_price_cents)
        .bind(clearing_cents)
        .bind(pi.geo_country.as_deref())
        .bind(pi.device_type.as_deref())
        .bind(pi.os.as_deref())
        .bind(pi.site_domain.as_deref())
        .execute(&state.pg)
        .await;

        if let Err(e) = result {
            warn!(error = %e, "failed to write impression event");
        }

        // Update campaign spend and deduct from advertiser wallet atomically.
        // GREATEST(0,...) prevents wallet going negative on any race condition.
        let spend_result = sqlx::query(
            r#"
            WITH updated_campaign AS (
                UPDATE campaigns
                SET spend_total_cents = spend_total_cents + $1
                WHERE id = $2
                RETURNING advertiser_id
            )
            UPDATE advertisers
            SET wallet_balance_cents = GREATEST(0, wallet_balance_cents - $1)
            WHERE id = (SELECT advertiser_id FROM updated_campaign)
            "#,
        )
        .bind(clearing_cents)
        .bind(pi.campaign_id)
        .execute(&state.pg)
        .await;

        if let Err(e) = spend_result {
            warn!(error = %e, "failed to update campaign spend / wallet");
        }
    }

    StatusCode::OK
}
