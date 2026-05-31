use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Advertiser ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Advertiser {
    pub id: Uuid,
    pub email: String,
    pub company_name: String,
    pub wallet_balance_cents: i64,
    #[serde(skip)]
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdvertiserRequest {
    pub email: String,
    pub company_name: String,
}

#[derive(Debug, Deserialize)]
pub struct TopUpRequest {
    /// Amount to add in US cents (e.g. 10000 = $100.00)
    pub amount_cents: i64,
}

// ── Campaign ──────────────────────────────────────────────────────────────────

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
    pub frequency_cap_daily: Option<i32>,
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
    pub budget_total_cents: i64,
    pub budget_daily_cents: Option<i64>,
    pub frequency_cap_daily: Option<i32>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCampaignRequest {
    pub name: Option<String>,
    pub bid_price_cpm_cents: Option<i64>,
    pub budget_total_cents: Option<i64>,
    pub budget_daily_cents: Option<i64>,
    pub frequency_cap_daily: Option<i32>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
}

// ── Targeting ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Targeting {
    pub campaign_id: Uuid,
    /// ISO-3166-1-alpha-3 codes: ["ARE","SAU","KWT","QAT","BHR","OMN"]
    pub geo_countries: Vec<String>,
    /// ["mobile","desktop","tablet"]
    pub device_types: Vec<String>,
    /// ["ios","android","windows","macos"]
    pub os_types: Vec<String>,
    /// IAB taxonomy codes: ["IAB13","IAB19"]
    pub site_categories: Vec<String>,
    /// BCP-47 codes: ["ar","en"]
    pub languages: Vec<String>,
    /// UTC hours 0–23 (empty = all hours)
    pub hours_of_day: Vec<i32>,
    /// 0=Sunday…6=Saturday (empty = all days)
    pub days_of_week: Vec<i32>,
    /// Only bid on these domains (empty = all domains)
    pub domain_allowlist: Vec<String>,
    /// Never bid on these domains
    pub domain_blocklist: Vec<String>,
    /// Contextual keywords matched against site.keywords in bid request (empty = all)
    pub keywords: Vec<String>,
    /// Minimum age (matched against user.yob when exchange provides it)
    pub age_min: Option<i32>,
    /// Maximum age
    pub age_max: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTargetingRequest {
    pub geo_countries: Option<Vec<String>>,
    pub device_types: Option<Vec<String>>,
    pub os_types: Option<Vec<String>>,
    pub site_categories: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
    pub hours_of_day: Option<Vec<i32>>,
    pub days_of_week: Option<Vec<i32>>,
    pub domain_allowlist: Option<Vec<String>>,
    pub domain_blocklist: Option<Vec<String>>,
    pub keywords: Option<Vec<String>>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
}

// ── Creative ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Creative {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub format: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// Main image / video URL
    pub asset_url: String,
    pub click_url: String,
    pub status: String,
    // Native-only fields
    pub title_text: Option<String>,
    pub description: Option<String>,
    pub cta_text: Option<String>,
    pub sponsored_by: Option<String>,
    pub created_at: DateTime<Utc>,
    // Optimizer fields
    pub parent_creative_id: Option<Uuid>,
    pub variant_hypothesis: Option<String>,
    // Live stats (joined from event tables)
    #[sqlx(default)]
    pub impressions: i64,
    #[sqlx(default)]
    pub clicks: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateCreativeRequest {
    /// "banner" | "native" | "video"
    pub format: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub asset_url: String,
    pub click_url: String,
    // Native-only
    pub title_text: Option<String>,
    pub description: Option<String>,
    pub cta_text: Option<String>,
    pub sponsored_by: Option<String>,
}

// ── Exchange config ───────────────────────────────────────────────────────────

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ExchangeConfig {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub status: String,
    pub endpoint_url: Option<String>,
    pub win_price_macro: String,
    pub min_floor_cents: i32,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExchangeRequest {
    pub slug: String,
    pub name: String,
    pub status: Option<String>,
    pub endpoint_url: Option<String>,
    pub win_price_macro: Option<String>,
    pub min_floor_cents: Option<i32>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateExchangeRequest {
    pub name: Option<String>,
    pub status: Option<String>,
    pub endpoint_url: Option<String>,
    pub win_price_macro: Option<String>,
    pub min_floor_cents: Option<i32>,
    pub notes: Option<String>,
}

// ── Reporting ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct CampaignReport {
    pub campaign_id: Uuid,
    pub from: String,
    pub to: String,
    pub impressions: i64,
    pub spend_cents: i64,
    pub clicks: i64,
    pub ctr_pct: f64,
    pub avg_cpm_cents: i64,
    pub conversions: i64,
    pub conversion_value_cents: i64,
    pub daily: Vec<DailyStats>,
}

#[derive(Debug, Serialize)]
pub struct DailyStats {
    pub date: String,
    pub impressions: i64,
    pub spend_cents: i64,
    pub clicks: i64,
    pub conversions: i64,
    pub conversion_value_cents: i64,
}
