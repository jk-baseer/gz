/// Event consumer — syncs PostgreSQL events → ClickHouse for analytics at scale.
///
/// Runs every 60 seconds. Uses ch_sync_cursors table to track last synced
/// timestamp per topic. On each tick:
///   1. Reads new rows from PostgreSQL since last sync
///   2. POSTs them to ClickHouse via HTTP (JSONEachRow format)
///   3. Advances the cursor on success
///
/// If CLICKHOUSE_URL is not reachable, the service logs warnings and retries
/// next tick — PostgreSQL remains the source of truth.
use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::postgres::PgPoolOptions;
use sqlx::Row;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = common::Config::load()?;
    common::telemetry::init(&cfg.log_level);

    let pg = PgPoolOptions::new()
        .max_connections(3)
        .connect(&cfg.database_url)
        .await
        .context("connecting to PostgreSQL")?;

    sqlx::migrate!("../../migrations").run(&pg).await?;

    let http = reqwest::Client::new();

    // Bootstrap ClickHouse schema (idempotent)
    if let Err(e) = bootstrap_clickhouse(&http, &cfg.clickhouse_url, &cfg.clickhouse_db).await {
        warn!(error = %e, "ClickHouse schema init failed — will retry next tick");
    } else {
        info!("ClickHouse schema ready");
    }

    info!(
        clickhouse = %cfg.clickhouse_url,
        db = %cfg.clickhouse_db,
        "event-consumer started — syncing every 60s"
    );

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
    loop {
        interval.tick().await;

        if let Err(e) = sync_impressions(&pg, &http, &cfg.clickhouse_url, &cfg.clickhouse_db).await {
            error!(error = %e, "impression sync failed");
        }
        if let Err(e) = sync_clicks(&pg, &http, &cfg.clickhouse_url, &cfg.clickhouse_db).await {
            error!(error = %e, "click sync failed");
        }
        if let Err(e) = sync_conversions(&pg, &http, &cfg.clickhouse_url, &cfg.clickhouse_db).await {
            error!(error = %e, "conversion sync failed");
        }
    }
}

async fn bootstrap_clickhouse(http: &reqwest::Client, url: &str, db: &str) -> anyhow::Result<()> {
    let stmts = [
        format!("CREATE DATABASE IF NOT EXISTS {db}"),
        format!(
            r#"CREATE TABLE IF NOT EXISTS {db}.impression_events (
                id                   UUID,
                campaign_id          UUID,
                creative_id          UUID,
                exchange             String,
                auction_id           String,
                bid_price_cents      Int64,
                clearing_price_cents Int64,
                geo_country          String,
                device_type          String,
                os                   String,
                site_domain          String,
                created_at           DateTime64(3, 'UTC'),
                viewed_at            Nullable(DateTime64(3, 'UTC'))
            ) ENGINE = MergeTree() ORDER BY (campaign_id, created_at)"#
        ),
        format!(
            r#"CREATE TABLE IF NOT EXISTS {db}.click_events (
                id           UUID,
                campaign_id  UUID,
                creative_id  UUID,
                auction_id   String,
                exchange     String,
                created_at   DateTime64(3, 'UTC')
            ) ENGINE = MergeTree() ORDER BY (campaign_id, created_at)"#
        ),
        format!(
            r#"CREATE TABLE IF NOT EXISTS {db}.conversion_events (
                id           UUID,
                campaign_id  UUID,
                value_cents  Int64,
                created_at   DateTime64(3, 'UTC')
            ) ENGINE = MergeTree() ORDER BY (campaign_id, created_at)"#
        ),
    ];

    for stmt in &stmts {
        ch_exec(http, url, stmt).await?;
    }
    Ok(())
}

async fn sync_impressions(
    pg: &sqlx::PgPool,
    http: &reqwest::Client,
    ch_url: &str,
    db: &str,
) -> anyhow::Result<()> {
    let cursor: DateTime<Utc> = sqlx::query(
        "SELECT last_synced_at FROM ch_sync_cursors WHERE topic = 'impressions'",
    )
    .fetch_one(pg)
    .await?
    .try_get("last_synced_at")?;

    let rows = sqlx::query(
        r#"
        SELECT id, campaign_id, creative_id,
               COALESCE(exchange,'')      AS exchange,
               COALESCE(auction_id,'')    AS auction_id,
               bid_price_cents,
               COALESCE(clearing_price_cents, 0) AS clearing_price_cents,
               COALESCE(geo_country,'')   AS geo_country,
               COALESCE(device_type,'')   AS device_type,
               COALESCE(os,'')            AS os,
               COALESCE(site_domain,'')   AS site_domain,
               created_at,
               viewed_at
        FROM impression_events
        WHERE created_at > $1
        ORDER BY created_at
        LIMIT 5000
        "#,
    )
    .bind(cursor)
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut payload = String::new();
    let mut latest = cursor;

    for row in &rows {
        let id: uuid::Uuid = row.try_get("id")?;
        let campaign_id: uuid::Uuid = row.try_get("campaign_id")?;
        let creative_id: uuid::Uuid = row.try_get("creative_id")?;
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let viewed_at: Option<DateTime<Utc>> = row.try_get("viewed_at")?;

        let line = serde_json::json!({
            "id":                   id,
            "campaign_id":          campaign_id,
            "creative_id":          creative_id,
            "exchange":             row.try_get::<String,_>("exchange")?,
            "auction_id":           row.try_get::<String,_>("auction_id")?,
            "bid_price_cents":      row.try_get::<i64,_>("bid_price_cents")?,
            "clearing_price_cents": row.try_get::<i64,_>("clearing_price_cents")?,
            "geo_country":          row.try_get::<String,_>("geo_country")?,
            "device_type":          row.try_get::<String,_>("device_type")?,
            "os":                   row.try_get::<String,_>("os")?,
            "site_domain":          row.try_get::<String,_>("site_domain")?,
            "created_at":           created_at.timestamp_millis(),
            "viewed_at":            viewed_at.map(|t| t.timestamp_millis()),
        });
        payload.push_str(&line.to_string());
        payload.push('\n');
        if created_at > latest {
            latest = created_at;
        }
    }

    ch_insert(http, ch_url, db, "impression_events", &payload).await?;

    sqlx::query(
        "UPDATE ch_sync_cursors SET last_synced_at = $1 WHERE topic = 'impressions'",
    )
    .bind(latest)
    .execute(pg)
    .await?;

    info!(count = rows.len(), "synced impressions → ClickHouse");
    Ok(())
}

async fn sync_clicks(
    pg: &sqlx::PgPool,
    http: &reqwest::Client,
    ch_url: &str,
    db: &str,
) -> anyhow::Result<()> {
    let cursor: DateTime<Utc> = sqlx::query(
        "SELECT last_synced_at FROM ch_sync_cursors WHERE topic = 'clicks'",
    )
    .fetch_one(pg)
    .await?
    .try_get("last_synced_at")?;

    let rows = sqlx::query(
        r#"
        SELECT id, campaign_id, creative_id,
               COALESCE(auction_id,'') AS auction_id,
               COALESCE(exchange,'')   AS exchange,
               created_at
        FROM click_events
        WHERE created_at > $1
        ORDER BY created_at
        LIMIT 5000
        "#,
    )
    .bind(cursor)
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut payload = String::new();
    let mut latest = cursor;

    for row in &rows {
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let line = serde_json::json!({
            "id":          row.try_get::<uuid::Uuid,_>("id")?,
            "campaign_id": row.try_get::<uuid::Uuid,_>("campaign_id")?,
            "creative_id": row.try_get::<uuid::Uuid,_>("creative_id")?,
            "auction_id":  row.try_get::<String,_>("auction_id")?,
            "exchange":    row.try_get::<String,_>("exchange")?,
            "created_at":  created_at.timestamp_millis(),
        });
        payload.push_str(&line.to_string());
        payload.push('\n');
        if created_at > latest {
            latest = created_at;
        }
    }

    ch_insert(http, ch_url, db, "click_events", &payload).await?;

    sqlx::query(
        "UPDATE ch_sync_cursors SET last_synced_at = $1 WHERE topic = 'clicks'",
    )
    .bind(latest)
    .execute(pg)
    .await?;

    info!(count = rows.len(), "synced clicks → ClickHouse");
    Ok(())
}

async fn sync_conversions(
    pg: &sqlx::PgPool,
    http: &reqwest::Client,
    ch_url: &str,
    db: &str,
) -> anyhow::Result<()> {
    let cursor: DateTime<Utc> = sqlx::query(
        "SELECT last_synced_at FROM ch_sync_cursors WHERE topic = 'conversions'",
    )
    .fetch_one(pg)
    .await?
    .try_get("last_synced_at")?;

    let rows = sqlx::query(
        r#"
        SELECT id, campaign_id, value_cents, created_at
        FROM conversion_events
        WHERE created_at > $1
        ORDER BY created_at
        LIMIT 5000
        "#,
    )
    .bind(cursor)
    .fetch_all(pg)
    .await?;

    if rows.is_empty() {
        return Ok(());
    }

    let mut payload = String::new();
    let mut latest = cursor;

    for row in &rows {
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let line = serde_json::json!({
            "id":          row.try_get::<uuid::Uuid,_>("id")?,
            "campaign_id": row.try_get::<uuid::Uuid,_>("campaign_id")?,
            "value_cents": row.try_get::<i64,_>("value_cents")?,
            "created_at":  created_at.timestamp_millis(),
        });
        payload.push_str(&line.to_string());
        payload.push('\n');
        if created_at > latest {
            latest = created_at;
        }
    }

    ch_insert(http, ch_url, db, "conversion_events", &payload).await?;

    sqlx::query(
        "UPDATE ch_sync_cursors SET last_synced_at = $1 WHERE topic = 'conversions'",
    )
    .bind(latest)
    .execute(pg)
    .await?;

    info!(count = rows.len(), "synced conversions → ClickHouse");
    Ok(())
}

/// Execute a DDL statement against ClickHouse via HTTP.
async fn ch_exec(http: &reqwest::Client, url: &str, stmt: &str) -> anyhow::Result<()> {
    let resp = http
        .post(url)
        .query(&[("query", stmt)])
        .send()
        .await
        .context("ClickHouse DDL request")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("ClickHouse DDL failed: {body}");
    }
    Ok(())
}

/// Bulk-insert rows into ClickHouse using JSONEachRow format.
async fn ch_insert(
    http: &reqwest::Client,
    url: &str,
    db: &str,
    table: &str,
    payload: &str,
) -> anyhow::Result<()> {
    let query = format!(
        "INSERT INTO {db}.{table} FORMAT JSONEachRow"
    );
    let resp = http
        .post(url)
        .query(&[
            ("query", query.as_str()),
            ("input_format_skip_unknown_fields", "1"),
            ("date_time_input_format", "best_effort"),
        ])
        .header("Content-Type", "application/json")
        .body(payload.to_owned())
        .send()
        .await
        .context("ClickHouse insert request")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("ClickHouse insert failed: {body}");
    }
    Ok(())
}
