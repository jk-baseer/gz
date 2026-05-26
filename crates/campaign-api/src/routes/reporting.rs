use std::sync::Arc;
use sqlx::Row;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{models::{CampaignReport, DailyStats}, state::AppState};

#[derive(Deserialize)]
pub struct ReportQuery {
    /// "YYYY-MM-DD" or full ISO-8601
    from: String,
    to: String,
}

fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    // Try full ISO-8601 first, then date-only (treat as start of day UTC)
    s.parse::<DateTime<Utc>>().ok().or_else(|| {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| Utc.from_utc_datetime(&dt))
    })
}

pub async fn campaign_report(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    Query(q): Query<ReportQuery>,
) -> impl IntoResponse {
    let (Some(from), Some(to)) = (parse_date(&q.from), parse_date(&q.to)) else {
        return (StatusCode::BAD_REQUEST, "invalid date format, use YYYY-MM-DD").into_response();
    };
    // `to` is inclusive end-of-day: advance by 1 day
    let to_exclusive = to + chrono::Duration::days(1);

    // ── Totals ────────────────────────────────────────────────────────────────
    let totals = sqlx::query(
        r#"
        SELECT
            COUNT(*)                                        AS impressions,
            COALESCE(SUM(clearing_price_cents), 0)::BIGINT  AS spend_cents
        FROM impression_events
        WHERE campaign_id = $1
          AND created_at >= $2 AND created_at < $3
        "#,
    )
    .bind(campaign_id).bind(from).bind(to_exclusive)
    .fetch_one(&state.pg).await;

    let Ok(totals) = totals else {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    };

    let impressions: i64 = totals.try_get("impressions").unwrap_or(0);
    let spend_cents: i64 = totals.try_get("spend_cents").unwrap_or(0);

    let clicks: i64 = sqlx::query(
        "SELECT COUNT(*) AS clicks FROM click_events
         WHERE campaign_id = $1 AND created_at >= $2 AND created_at < $3",
    )
    .bind(campaign_id).bind(from).bind(to_exclusive)
    .fetch_one(&state.pg).await
    .ok().and_then(|r| r.try_get("clicks").ok()).unwrap_or(0);

    let (conversions, conversion_value_cents): (i64, i64) = sqlx::query(
        r#"SELECT COUNT(*) AS conversions,
                  COALESCE(SUM(value_cents), 0)::BIGINT AS conversion_value_cents
           FROM conversion_events
           WHERE campaign_id = $1 AND created_at >= $2 AND created_at < $3"#,
    )
    .bind(campaign_id).bind(from).bind(to_exclusive)
    .fetch_one(&state.pg).await
    .map(|r| (
        r.try_get("conversions").unwrap_or(0),
        r.try_get("conversion_value_cents").unwrap_or(0),
    ))
    .unwrap_or((0, 0));

    let ctr_pct = if impressions > 0 { clicks as f64 / impressions as f64 * 100.0 } else { 0.0 };
    let avg_cpm_cents = if impressions > 0 { spend_cents * 1000 / impressions } else { 0 };

    // ── Daily breakdown (CTEs avoid the cross-join bug) ───────────────────────
    let daily_rows = sqlx::query(
        r#"
        WITH
        imp AS (
            SELECT DATE_TRUNC('day', created_at) AS day,
                   COUNT(*)                                       AS impressions,
                   COALESCE(SUM(clearing_price_cents),0)::BIGINT  AS spend_cents
            FROM impression_events
            WHERE campaign_id = $1 AND created_at >= $2 AND created_at < $3
            GROUP BY 1
        ),
        clk AS (
            SELECT DATE_TRUNC('day', created_at) AS day, COUNT(*) AS clicks
            FROM click_events
            WHERE campaign_id = $1 AND created_at >= $2 AND created_at < $3
            GROUP BY 1
        ),
        conv AS (
            SELECT DATE_TRUNC('day', created_at) AS day,
                   COUNT(*)                                AS conversions,
                   COALESCE(SUM(value_cents), 0)::BIGINT  AS conversion_value_cents
            FROM conversion_events
            WHERE campaign_id = $1 AND created_at >= $2 AND created_at < $3
            GROUP BY 1
        )
        SELECT
            TO_CHAR(i.day, 'YYYY-MM-DD')           AS date,
            i.impressions,
            i.spend_cents,
            COALESCE(c.clicks, 0)                  AS clicks,
            COALESCE(v.conversions, 0)             AS conversions,
            COALESCE(v.conversion_value_cents, 0)  AS conversion_value_cents
        FROM imp i
        LEFT JOIN clk  c ON c.day  = i.day
        LEFT JOIN conv v ON v.day  = i.day
        ORDER BY i.day
        "#,
    )
    .bind(campaign_id).bind(from).bind(to_exclusive)
    .fetch_all(&state.pg).await;

    let daily = match daily_rows {
        Ok(rows) => rows.iter().map(|r| DailyStats {
            date:                    r.try_get::<String,_>("date").unwrap_or_default(),
            impressions:             r.try_get("impressions").unwrap_or(0),
            spend_cents:             r.try_get("spend_cents").unwrap_or(0),
            clicks:                  r.try_get("clicks").unwrap_or(0),
            conversions:             r.try_get("conversions").unwrap_or(0),
            conversion_value_cents:  r.try_get("conversion_value_cents").unwrap_or(0),
        }).collect(),
        Err(_) => vec![],
    };

    Json(CampaignReport {
        campaign_id,
        from: q.from,
        to: q.to,
        impressions,
        spend_cents,
        clicks,
        ctr_pct,
        avg_cpm_cents,
        conversions,
        conversion_value_cents,
        daily,
    }).into_response()
}
