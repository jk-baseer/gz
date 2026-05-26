use std::sync::Arc;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CampaignRecord {
    pub id: Uuid,
    pub advertiser_id: Uuid,
    pub name: String,
    pub bid_price_cpm_cents: i64,
    pub budget_daily_cents: Option<i64>,
    pub budget_total_cents: i64,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    /// Max impressions per user per day (None = uncapped)
    pub frequency_cap_daily: Option<i32>,
    pub targeting: TargetingRecord,
    pub creatives: Vec<CreativeRecord>,
}

#[derive(Debug, Clone, Default)]
pub struct TargetingRecord {
    /// ISO-3166-1-alpha-3 country codes (e.g. "ARE", "SAU")
    pub geo_countries: Vec<String>,
    /// "mobile", "desktop", "tablet"
    pub device_types: Vec<String>,
    /// "ios", "android", "windows", "macos"
    pub os_types: Vec<String>,
    /// IAB taxonomy codes (e.g. "IAB13")
    pub site_categories: Vec<String>,
    /// BCP-47 language codes (e.g. "ar", "en")
    pub languages: Vec<String>,
    /// 0–23
    pub hours_of_day: Vec<i32>,
    /// 0=Sunday … 6=Saturday
    pub days_of_week: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct CreativeRecord {
    pub id: Uuid,
    pub format: String,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub asset_url: String,
    pub click_url: String,
    pub title_text: Option<String>,
    pub description: Option<String>,
    pub cta_text: Option<String>,
    pub sponsored_by: Option<String>,
}

pub struct CampaignIndex {
    campaigns: RwLock<Vec<CampaignRecord>>,
}

impl CampaignIndex {
    pub fn new() -> Self {
        Self {
            campaigns: RwLock::new(vec![]),
        }
    }

    pub async fn get_all(&self) -> Vec<CampaignRecord> {
        self.campaigns.read().await.clone()
    }

    pub async fn refresh(&self, pg: &PgPool) -> anyhow::Result<()> {
        let campaigns = load_active_campaigns(pg).await?;
        let count = campaigns.len();
        *self.campaigns.write().await = campaigns;
        info!(count, "campaign index refreshed");
        Ok(())
    }
}

pub fn start_refresh_task(index: Arc<CampaignIndex>, pg: PgPool) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        interval.tick().await; // skip first tick (already refreshed at startup)
        loop {
            interval.tick().await;
            if let Err(e) = index.refresh(&pg).await {
                error!(error = %e, "campaign index refresh failed");
            }
        }
    });
}

async fn load_active_campaigns(pg: &PgPool) -> anyhow::Result<Vec<CampaignRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT
            c.id,
            c.advertiser_id,
            c.name,
            c.bid_price_cpm_cents,
            c.budget_daily_cents,
            c.budget_total_cents,
            c.start_date,
            c.end_date,
            c.frequency_cap_daily,
            COALESCE(t.geo_countries,   '{}') AS geo_countries,
            COALESCE(t.device_types,    '{}') AS device_types,
            COALESCE(t.os_types,        '{}') AS os_types,
            COALESCE(t.site_categories, '{}') AS site_categories,
            COALESCE(t.languages,       '{}') AS languages,
            COALESCE(t.hours_of_day,    '{}') AS hours_of_day,
            COALESCE(t.days_of_week,    '{}') AS days_of_week
        FROM campaigns c
        JOIN campaign_targeting t ON t.campaign_id = c.id
        WHERE c.status = 'active'
          AND (c.end_date IS NULL OR c.end_date > NOW())
        "#,
    )
    .fetch_all(pg)
    .await?;

    let mut campaigns = Vec::with_capacity(rows.len());
    for row in rows {
        let id: Uuid = row.try_get("id")?;
        let creatives = load_creatives(pg, id).await?;
        campaigns.push(CampaignRecord {
            id,
            advertiser_id: row.try_get("advertiser_id")?,
            name: row.try_get("name")?,
            bid_price_cpm_cents: row.try_get("bid_price_cpm_cents")?,
            budget_daily_cents: row.try_get("budget_daily_cents")?,
            budget_total_cents: row.try_get("budget_total_cents")?,
            start_date: row.try_get("start_date")?,
            end_date: row.try_get("end_date")?,
            frequency_cap_daily: row.try_get("frequency_cap_daily")?,
            targeting: TargetingRecord {
                geo_countries: row.try_get("geo_countries")?,
                device_types: row.try_get("device_types")?,
                os_types: row.try_get("os_types")?,
                site_categories: row.try_get("site_categories")?,
                languages: row.try_get("languages")?,
                hours_of_day: row.try_get("hours_of_day")?,
                days_of_week: row.try_get("days_of_week")?,
            },
            creatives,
        });
    }

    Ok(campaigns)
}

async fn load_creatives(pg: &PgPool, campaign_id: Uuid) -> anyhow::Result<Vec<CreativeRecord>> {
    let rows = sqlx::query(
        r#"
        SELECT id, format, width, height, asset_url, click_url,
               title_text, description, cta_text, sponsored_by
        FROM creatives
        WHERE campaign_id = $1 AND status = 'approved'
        "#,
    )
    .bind(campaign_id)
    .fetch_all(pg)
    .await?;

    rows.into_iter()
        .map(|r| {
            Ok(CreativeRecord {
                id: r.try_get("id")?,
                format: r.try_get("format")?,
                width: r.try_get("width")?,
                height: r.try_get("height")?,
                asset_url: r.try_get("asset_url")?,
                click_url: r.try_get("click_url")?,
                title_text: r.try_get("title_text")?,
                description: r.try_get("description")?,
                cta_text: r.try_get("cta_text")?,
                sponsored_by: r.try_get("sponsored_by")?,
            })
        })
        .collect()
}
