use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{models::{Creative, CreateCreativeRequest}, state::AppState};

pub async fn create(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    Json(body): Json<CreateCreativeRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Creative>(
        r#"
        INSERT INTO creatives
            (campaign_id, format, width, height, asset_url, click_url,
             title_text, description, cta_text, sponsored_by)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(campaign_id)
    .bind(&body.format)
    .bind(body.width)
    .bind(body.height)
    .bind(&body.asset_url)
    .bind(&body.click_url)
    .bind(&body.title_text)
    .bind(&body.description)
    .bind(&body.cta_text)
    .bind(&body.sponsored_by)
    .fetch_one(&state.pg)
    .await;

    match result {
        Ok(c) => (StatusCode::CREATED, Json(c)).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to create creative");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Creative>(
        r#"
        SELECT c.*,
               COALESCE(i.impressions, 0) AS impressions,
               COALESCE(cl.clicks, 0)    AS clicks
        FROM creatives c
        LEFT JOIN (SELECT creative_id, COUNT(*)::BIGINT AS impressions FROM impression_events GROUP BY creative_id) i
               ON i.creative_id = c.id
        LEFT JOIN (SELECT creative_id, COUNT(*)::BIGINT AS clicks FROM click_events GROUP BY creative_id) cl
               ON cl.creative_id = c.id
        WHERE c.campaign_id = $1
        ORDER BY c.created_at DESC
        "#,
    )
    .bind(campaign_id)
    .fetch_all(&state.pg)
    .await;

    match result {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to list creatives");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn reject(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query("UPDATE creatives SET status = 'rejected' WHERE id = $1")
        .bind(id)
        .execute(&state.pg)
        .await;

    match result {
        Ok(r) if r.rows_affected() == 0 => StatusCode::NOT_FOUND.into_response(),
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to reject creative");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn approve(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query(
        "UPDATE creatives SET status = 'approved' WHERE id = $1"
    )
    .bind(id)
    .execute(&state.pg)
    .await;

    match result {
        Ok(r) if r.rows_affected() == 0 => StatusCode::NOT_FOUND.into_response(),
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to approve creative");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
