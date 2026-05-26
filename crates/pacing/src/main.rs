/// Pacing service — smooths budget spend across the campaign flight.
///
/// Every minute it calculates the ideal per-minute spend cap for each active
/// campaign and writes it to Redis. The bidder checks this cap before placing
/// bids to avoid budget exhausting in the first hour of the day.
///
/// Algorithm: pace_cents_per_minute = remaining_daily_budget / remaining_minutes_today

use chrono::{Timelike, Utc};
use redis::AsyncCommands;
use sqlx::postgres::PgPoolOptions;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);

    let pg = PgPoolOptions::new()
        .max_connections(3)
        .connect(&cfg.database_url)
        .await?;

    let redis = redis::Client::open(cfg.redis_url.as_str())?;
    let mut redis_mgr = redis::aio::ConnectionManager::new(redis).await?;

    info!("pacing service started — updating caps every 60s");

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        interval.tick().await;
        if let Err(e) = update_pace_caps(&pg, &mut redis_mgr).await {
            error!(error = %e, "pacing update failed");
        }
    }
}

async fn update_pace_caps(
    pg: &sqlx::PgPool,
    redis: &mut redis::aio::ConnectionManager,
) -> anyhow::Result<()> {
    let now = Utc::now();
    let today = now.format("%Y-%m-%d").to_string();

    // Minutes remaining in today (UTC)
    let remaining_minutes = {
        let elapsed = now.hour() * 60 + now.minute();
        (1440u32.saturating_sub(elapsed)).max(1) as i64
    };

    // Load all active campaigns with a daily budget
    let rows = sqlx::query(
        r#"
        SELECT id, budget_daily_cents
        FROM campaigns
        WHERE status = 'active'
          AND budget_daily_cents IS NOT NULL
          AND (end_date IS NULL OR end_date > NOW())
        "#,
    )
    .fetch_all(pg)
    .await?;

    for row in &rows {
        use sqlx::Row;
        let campaign_id: uuid::Uuid = row.try_get("id")?;
        let daily_budget: i64 = row.try_get("budget_daily_cents")?;

        let spend_key = format!("spend:daily:{campaign_id}:{today}");
        let spent: i64 = redis.get(&spend_key).await.unwrap_or(0);
        let remaining = (daily_budget - spent).max(0);

        let cap_per_minute = remaining / remaining_minutes;

        let cap_key = format!("pace_cap:{campaign_id}");
        let _: () = redis.set_ex(&cap_key, cap_per_minute, 120).await?;
    }

    info!(campaigns = rows.len(), remaining_minutes, "pace caps updated");
    Ok(())
}
