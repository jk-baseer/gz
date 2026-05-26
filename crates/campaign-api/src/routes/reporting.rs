use std::sync::Arc;
use sqlx::Row;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{models::{CampaignReport, DailyStats}, state::AppState};

#[derive(Deserialize)]
pub struct ReportQuery {
    /// ISO-8601 datetime (e.g. 2024-01-01T00:00:00Z)
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

pub async fn campaign_report(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    Query(q): Query<ReportQuery>,
) -> impl IntoResponse {
    // Impression totals
    let totals = sqlx::query(
        r#"
        SELECT
            COUNT(*) AS impressions,
            COALESCE(SUM(clearing_price_cents), 0) AS spend_cents
        FROM impression_events
        WHERE campaign_id = $1
          AND created_at >= $2
          AND created_at <  $3
        "#,
    )
    .bind(campaign_id)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pg)
    .await;

    let Ok(totals) = totals else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let impressions: i64 = totals.try_get("impressions").unwrap_or(0);
    let spend_cents: i64 = totals.try_get("spend_cents").unwrap_or(0);

    // Click totals
    let clicks_row = sqlx::query(
        r#"
        SELECT COUNT(*) AS clicks
        FROM click_events
        WHERE campaign_id = $1
          AND created_at >= $2
          AND created_at <  $3
        "#,
    )
    .bind(campaign_id)
    .bind(q.from)
    .bind(q.to)
    .fetch_one(&state.pg)
    .await;

    let clicks: i64 = clicks_row
        .ok()
        .and_then(|r| r.try_get("clicks").ok())
        .unwrap_or(0);

    let ctr_pct = if impressions > 0 {
        (clicks as f64 / impressions as f64) * 100.0
    } else {
        0.0
    };

    let avg_cpm_cents = if impressions > 0 {
        spend_cents / impressions
    } else {
        0
    };

    // Daily breakdown
    let daily_rows = sqlx::query(
        r#"
        SELECT
            TO_CHAR(DATE_TRUNC('day', ie.created_at), 'YYYY-MM-DD') AS date,
            COUNT(ie.id) AS impressions,
            COALESCE(SUM(ie.clearing_price_cents), 0) AS spend_cents,
            COUNT(ce.id) AS clicks
        FROM impression_events ie
        LEFT JOIN click_events ce
            ON ce.campaign_id = ie.campaign_id
           AND DATE_TRUNC('day', ce.created_at) = DATE_TRUNC('day', ie.created_at)
        WHERE ie.campaign_id = $1
          AND ie.created_at >= $2
          AND ie.created_at <  $3
        GROUP BY DATE_TRUNC('day', ie.created_at)
        ORDER BY DATE_TRUNC('day', ie.created_at)
        "#,
    )
    .bind(campaign_id)
    .bind(q.from)
    .bind(q.to)
    .fetch_all(&state.pg)
    .await;

    let daily = match daily_rows {
        Ok(rows) => rows
            .iter()
            .map(|r| DailyStats {
                date: r.try_get::<String, _>("date").unwrap_or_default(),
                impressions: r.try_get("impressions").unwrap_or(0),
                spend_cents: r.try_get("spend_cents").unwrap_or(0),
                clicks: r.try_get("clicks").unwrap_or(0),
            })
            .collect(),
        Err(_) => vec![],
    };

    Json(CampaignReport {
        campaign_id,
        from: q.from.to_rfc3339(),
        to: q.to.to_rfc3339(),
        impressions,
        spend_cents,
        clicks,
        ctr_pct,
        avg_cpm_cents,
        daily,
    })
    .into_response()
}
