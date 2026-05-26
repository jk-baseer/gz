use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{models::{Campaign, CreateCampaignRequest, UpdateCampaignRequest}, state::AppState};

#[derive(Deserialize, Default)]
pub struct CampaignListQuery {
    pub advertiser_id: Option<Uuid>,
}

pub async fn list(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CampaignListQuery>,
) -> impl IntoResponse {
    let result = match q.advertiser_id {
        Some(aid) => sqlx::query_as::<_, Campaign>(
            "SELECT * FROM campaigns WHERE advertiser_id = $1 ORDER BY created_at DESC",
        )
        .bind(aid)
        .fetch_all(&state.pg)
        .await,

        None => sqlx::query_as::<_, Campaign>(
            "SELECT * FROM campaigns ORDER BY created_at DESC",
        )
        .fetch_all(&state.pg)
        .await,
    };

    match result {
        Ok(campaigns) => Json(campaigns).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to list campaigns");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Campaign>("SELECT * FROM campaigns WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pg)
        .await;

    match result {
        Ok(Some(c)) => Json(c).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get campaign");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateCampaignRequest>,
) -> impl IntoResponse {
    let mut tx = match state.pg.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(error = %e, "failed to begin transaction");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let campaign = sqlx::query_as::<_, Campaign>(
        r#"
        INSERT INTO campaigns (
            advertiser_id, name, bid_price_cpm_cents,
            budget_total_cents, budget_daily_cents,
            frequency_cap_daily, start_date, end_date
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(body.advertiser_id)
    .bind(&body.name)
    .bind(body.bid_price_cpm_cents)
    .bind(body.budget_total_cents)
    .bind(body.budget_daily_cents)
    .bind(body.frequency_cap_daily)
    .bind(body.start_date)
    .bind(body.end_date)
    .fetch_one(&mut *tx)
    .await;

    let campaign = match campaign {
        Ok(c) => c,
        Err(e) => {
            tracing::error!(error = %e, "failed to create campaign");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    if let Err(e) = sqlx::query("INSERT INTO campaign_targeting (campaign_id) VALUES ($1)")
        .bind(campaign.id)
        .execute(&mut *tx)
        .await
    {
        tracing::error!(error = %e, "failed to create targeting record");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(error = %e, "transaction commit failed");
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    (StatusCode::CREATED, Json(campaign)).into_response()
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateCampaignRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Campaign>(
        r#"
        UPDATE campaigns SET
            name                = COALESCE($2, name),
            bid_price_cpm_cents = COALESCE($3, bid_price_cpm_cents),
            budget_total_cents  = COALESCE($4, budget_total_cents),
            budget_daily_cents  = COALESCE($5, budget_daily_cents),
            frequency_cap_daily = COALESCE($6, frequency_cap_daily),
            start_date          = COALESCE($7, start_date),
            end_date            = COALESCE($8, end_date),
            updated_at          = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(body.name)
    .bind(body.bid_price_cpm_cents)
    .bind(body.budget_total_cents)
    .bind(body.budget_daily_cents)
    .bind(body.frequency_cap_daily)
    .bind(body.start_date)
    .bind(body.end_date)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(c)) => Json(c).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to update campaign");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn pause(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    set_status(&state, id, "paused").await
}

pub async fn activate(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    set_status(&state, id, "active").await
}

async fn set_status(state: &Arc<AppState>, id: Uuid, status: &str) -> impl IntoResponse {
    let result = sqlx::query(
        "UPDATE campaigns SET status = $1, updated_at = NOW() WHERE id = $2",
    )
    .bind(status)
    .bind(id)
    .execute(&state.pg)
    .await;

    match result {
        Ok(r) if r.rows_affected() == 0 => StatusCode::NOT_FOUND.into_response(),
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to set campaign status");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
