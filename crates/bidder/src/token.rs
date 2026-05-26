/// Tracking tokens are base64url-encoded JSON blobs embedded in ad markup.
/// They carry the context needed to record click/impression events without
/// a database lookup on the hot path.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct TrackingToken {
    /// Our internal campaign ID
    pub campaign_id: Uuid,
    pub creative_id: Uuid,
    /// Final destination URL (only needed for click tokens)
    pub click_url: String,
    pub auction_id: String,
    pub bid_id: String,
    pub exchange: String,
}

pub fn encode(token: &TrackingToken) -> anyhow::Result<String> {
    let json = serde_json::to_string(token)?;
    Ok(URL_SAFE_NO_PAD.encode(json.as_bytes()))
}

pub fn decode(s: &str) -> anyhow::Result<TrackingToken> {
    let bytes = URL_SAFE_NO_PAD.decode(s)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Stored in Redis keyed by bid_id; retrieved when the win notice arrives.
/// Contains the geo/device context we captured at bid time.
#[derive(Debug, Serialize, Deserialize)]
pub struct PendingImpression {
    pub campaign_id: Uuid,
    pub creative_id: Uuid,
    pub bid_price_cents: i64,
    pub exchange: String,
    pub geo_country: Option<String>,
    pub device_type: Option<String>,
    pub os: Option<String>,
    pub site_domain: Option<String>,
}
