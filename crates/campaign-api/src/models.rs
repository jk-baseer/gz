use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Campaign {
    pub id: Uuid,
    pub advertiser_id: Uuid,
    pub name: String,
    pub status: String,
    pub bid_price_cpm_cents: i64,
    pub budget_total_cents: i64,
    pub budget_daily_cents: Option<i64>,
    pub spend_total_cents: i64,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCampaignRequest {
    pub advertiser_id: Uuid,
    pub name: String,
    /// CPM bid price in US cents (e.g. 200 = $2.00 CPM)
    pub bid_price_cpm_cents: i64,
    /// Total campaign budget in US cents
    pub budget_total_cents: i64,
    pub budget_daily_cents: Option<i64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub bid_price_cpm_cents: Option<i64>,
    pub budget_total_cents: Option<i64>,
    pub budget_daily_cents: Option<i64>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Creative {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub format: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub asset_url: String,
    pub click_url: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCreativeRequest {
    pub format: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub asset_url: String,
    pub click_url: String,
}
