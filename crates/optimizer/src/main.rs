mod bandit;

use anyhow::Result;
use common::Config;
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use tracing::{error, info};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::load()?;
    common::telemetry::init(&cfg.log_level);

    let pg = PgPoolOptions::new()
        .max_connections(5)
        .connect(&cfg.database_url)
        .await?;

    info!("optimizer started — running every 300 seconds");

    // Run once immediately at startup, then on the interval
    if let Err(e) = run_optimization(&pg).await {
        error!(error = %e, "initial optimization run failed");
    }

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
    interval.tick().await; // skip the first tick (already ran above)
    loop {
        interval.tick().await;
        if let Err(e) = run_optimization(&pg).await {
            error!(error = %e, "optimization run failed");
        }
    }
}

async fn run_optimization(pg: &PgPool) -> Result<()> {
    // Load all active campaigns
    let campaign_rows = sqlx::query(
        "SELECT id FROM campaigns WHERE status = 'active' AND (end_date IS NULL OR end_date > NOW())"
    )
    .fetch_all(pg)
    .await?;

    let mut total_weights_updated = 0usize;

    for row in &campaign_rows {
        let campaign_id: Uuid = row.try_get("id")?;
        if let Err(e) = optimize_campaign(pg, campaign_id, &mut total_weights_updated).await {
            error!(campaign_id = %campaign_id, error = %e, "failed to optimize campaign");
        }
    }

    info!(
        campaigns = campaign_rows.len(),
        weights_updated = total_weights_updated,
        "optimization run complete"
    );
    Ok(())
}

async fn optimize_campaign(
    pg: &PgPool,
    campaign_id: Uuid,
    total_updated: &mut usize,
) -> Result<()> {
    // Load approved creatives with their impression and click counts
    let rows = sqlx::query(
        r#"
        SELECT
            c.id AS creative_id,
            COALESCE(i.impressions, 0) AS impressions,
            COALESCE(cl.clicks, 0)     AS clicks
        FROM creatives c
        LEFT JOIN (
            SELECT creative_id, COUNT(*)::BIGINT AS impressions
            FROM impression_events
            GROUP BY creative_id
        ) i  ON i.creative_id  = c.id
        LEFT JOIN (
            SELECT creative_id, COUNT(*)::BIGINT AS clicks
            FROM click_events
            GROUP BY creative_id
        ) cl ON cl.creative_id = c.id
        WHERE c.campaign_id = $1
          AND c.status = 'approved'
        "#,
    )
    .bind(campaign_id)
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    // Build input for bandit
    let creatives: Vec<(Uuid, i64, i64)> = rows
        .iter()
        .map(|r| {
            let id: Uuid = r.try_get("creative_id").unwrap();
            let imp: i64 = r.try_get("impressions").unwrap_or(0);
            let cl:  i64 = r.try_get("clicks").unwrap_or(0);
            (id, imp, cl)
        })
        .collect();

    // Run Monte Carlo Thompson Sampling
    let allocations = bandit::compute_allocation_probabilities(&creatives);

    // Upsert results into creative_serving_weights
    for ((creative_id, impressions, clicks), (_, probability)) in creatives.iter().zip(&allocations) {
        sqlx::query(
            r#"
            INSERT INTO creative_serving_weights
                (creative_id, campaign_id, serving_probability, impressions_snapshot, clicks_snapshot, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            ON CONFLICT (creative_id) DO UPDATE SET
                serving_probability  = EXCLUDED.serving_probability,
                impressions_snapshot = EXCLUDED.impressions_snapshot,
                clicks_snapshot      = EXCLUDED.clicks_snapshot,
                updated_at           = NOW()
            "#,
        )
        .bind(creative_id)
        .bind(campaign_id)
        .bind(probability)
        .bind(impressions)
        .bind(clicks)
        .execute(pg)
        .await?;

        *total_updated += 1;
    }

    Ok(())
}
