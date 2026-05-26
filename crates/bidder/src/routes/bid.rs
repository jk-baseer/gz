use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use redis::AsyncCommands;
use tracing::{debug, warn};
use uuid::Uuid;

use openrtb::{BidRequest, BidResponse, Bid, SeatBid};
use bidder::{budget, targeting, targeting::device_type_str, token};
use crate::state::AppState;

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Path(exchange): Path<String>,
    Json(req): Json<BidRequest>,
) -> impl IntoResponse {
    debug!(auction_id = %req.id, exchange = %exchange, imps = req.imp.len(), "bid request");

    let campaigns = state.index.get_all().await;
    if campaigns.is_empty() {
        return StatusCode::NO_CONTENT.into_response();
    }

    // Extract context once for all impressions
    let geo_country = req.device.as_ref()
        .and_then(|d| d.geo.as_ref())
        .and_then(|g| g.country.as_deref())
        .map(String::from);

    let device_type = req.device.as_ref()
        .and_then(|d| d.devicetype)
        .map(device_type_str);

    let os = req.device.as_ref()
        .and_then(|d| d.os.as_deref())
        .map(String::from);

    let site_domain = req.site.as_ref()
        .and_then(|s| s.domain.as_deref())
        .or_else(|| req.app.as_ref().and_then(|a| a.domain.as_deref()))
        .map(String::from);

    let mut bids: Vec<Bid> = Vec::new();
    let mut redis = state.redis.clone();

    for imp in &req.imp {
        let winner = campaigns
            .iter()
            .filter(|c| targeting::matches(c, &req, imp))
            .max_by_key(|c| c.bid_price_cpm_cents);

        let Some(campaign) = winner else {
            debug!(imp_id = %imp.id, "no matching campaign");
            continue;
        };

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

        // Store full impression context in Redis (TTL 1h); retrieved on win notice
        let pending = token::PendingImpression {
            campaign_id: campaign.id,
            creative_id: creative.id,
            bid_price_cents: campaign.bid_price_cpm_cents,
            exchange: exchange.clone(),
            geo_country: geo_country.clone(),
            device_type: device_type.clone(),
            os: os.clone(),
            site_domain: site_domain.clone(),
        };
        let redis_key = format!("pending_imp:{bid_id}");
        if let Ok(json) = serde_json::to_string(&pending) {
            let _: Result<(), _> = redis.set_ex(&redis_key, json, 3600).await;
        }

        // Build tracking tokens for ad markup
        let tracking = token::TrackingToken {
            campaign_id: campaign.id,
            creative_id: creative.id,
            click_url: creative.click_url.clone(),
            auction_id: req.id.clone(),
            bid_id: bid_id.clone(),
            exchange: exchange.clone(),
        };
        let tok = token::encode(&tracking).unwrap_or_default();

        let win_url = format!(
            "https://{host}/win?aid={aid}&bid={bid_id}&cid={cid}&exchange={ex}&price=${{AUCTION_PRICE}}",
            host   = state.cfg.public_hostname,
            aid    = req.id,
            cid    = campaign.id,
            ex     = exchange,
        );

        let ad_markup = banner_markup(
            &creative.asset_url,
            &tok,
            &state.cfg.public_hostname,
            creative.width,
            creative.height,
        );

        bids.push(Bid {
            id: bid_id,
            impid: imp.id.clone(),
            price: campaign.bid_price_cpm_cents as f64 / 100.0,
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

fn banner_markup(
    asset_url: &str,
    token: &str,
    hostname: &str,
    w: Option<i32>,
    h: Option<i32>,
) -> String {
    let click_url = format!("https://{hostname}/click/{token}");
    let imp_url = format!("https://{hostname}/imp/{token}");
    let width = w.unwrap_or(0);
    let height = h.unwrap_or(0);
    format!(
        r#"<a href="{click_url}" target="_blank" rel="noopener"><img src="{asset_url}" width="{width}" height="{height}" border="0" alt="Advertisement"/></a><img src="{imp_url}" width="1" height="1" border="0" style="display:none" alt=""/>"#
    )
}
