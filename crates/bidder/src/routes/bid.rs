use std::sync::Arc;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::{debug, warn};
use uuid::Uuid;

use openrtb::{BidRequest, BidResponse, Bid, SeatBid};
use crate::{budget, targeting, state::AppState};

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BidRequest>,
) -> impl IntoResponse {
    debug!(auction_id = %req.id, imps = req.imp.len(), "bid request received");

    let campaigns = state.index.get_all().await;
    if campaigns.is_empty() {
        return StatusCode::NO_CONTENT.into_response();
    }

    let mut bids: Vec<Bid> = Vec::new();
    let mut redis = state.redis.clone();

    for imp in &req.imp {
        // Find the highest-bidding campaign that matches this impression
        let winner = campaigns
            .iter()
            .filter(|c| targeting::matches(c, &req, imp))
            .max_by_key(|c| c.bid_price_cpm_cents);

        let Some(campaign) = winner else {
            debug!(imp_id = %imp.id, "no matching campaign");
            continue;
        };

        let bid_price_cpm = campaign.bid_price_cpm_cents as f64 / 100.0;

        // Budget check (atomic Redis Lua script)
        let approved = match budget::try_reserve(
            &mut redis,
            campaign.id,
            campaign.bid_price_cpm_cents,
            campaign.budget_daily_cents,
            campaign.budget_total_cents,
        )
        .await
        {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, campaign_id = %campaign.id, "budget check failed");
                continue;
            }
        };

        if !approved {
            debug!(campaign_id = %campaign.id, "budget exhausted");
            continue;
        }

        let Some(creative) = targeting::pick_creative(campaign, imp) else {
            continue;
        };

        let bid_id = Uuid::new_v4().to_string();
        let win_url = format!(
            "https://{}/win?aid={}&bid={}&cid={}&price=${{AUCTION_PRICE}}",
            state.cfg.public_hostname,
            req.id,
            bid_id,
            campaign.id,
        );

        let ad_markup = banner_markup(&creative.asset_url, &creative.click_url, creative.width, creative.height);

        bids.push(Bid {
            id: bid_id,
            impid: imp.id.clone(),
            price: bid_price_cpm,
            adid: Some(campaign.id.to_string()),
            nurl: Some(win_url),
            burl: None,
            adm: Some(ad_markup),
            adomain: None,
            crid: Some(creative.id.to_string()),
            cat: None,
            w: creative.width.map(|w| w as u32),
            h: creative.height.map(|h| h as u32),
            exp: Some(300),
            ext: None,
        });
    }

    if bids.is_empty() {
        return StatusCode::NO_CONTENT.into_response();
    }

    let response = BidResponse {
        id: req.id.clone(),
        seatbid: vec![SeatBid {
            bid: bids,
            seat: Some("gz".to_string()),
            group: None,
        }],
        bidid: Some(Uuid::new_v4().to_string()),
        cur: Some("USD".to_string()),
        nbr: None,
        ext: None,
    };

    Json(response).into_response()
}

fn banner_markup(asset_url: &str, click_url: &str, w: Option<i32>, h: Option<i32>) -> String {
    let width = w.unwrap_or(0);
    let height = h.unwrap_or(0);
    format!(
        r#"<a href="{click_url}" target="_blank" rel="noopener">
  <img src="{asset_url}" width="{width}" height="{height}" border="0" alt="Advertisement" />
</a>"#
    )
}
