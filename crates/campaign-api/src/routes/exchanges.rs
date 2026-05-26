use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    models::{ExchangeConfig, CreateExchangeRequest, UpdateExchangeRequest},
    state::AppState,
};

pub async fn list(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ExchangeConfig>(
        "SELECT * FROM exchange_configs ORDER BY name",
    )
    .fetch_all(&state.pg)
    .await;

    match result {
        Ok(rows) => Json(rows).into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to list exchanges");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ExchangeConfig>(
        "SELECT * FROM exchange_configs WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(e)) => Json(e).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to get exchange");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateExchangeRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ExchangeConfig>(
        r#"
        INSERT INTO exchange_configs
            (slug, name, status, endpoint_url, win_price_macro, min_floor_cents, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(&body.slug)
    .bind(&body.name)
    .bind(body.status.as_deref().unwrap_or("testing"))
    .bind(&body.endpoint_url)
    .bind(body.win_price_macro.as_deref().unwrap_or("${AUCTION_PRICE}"))
    .bind(body.min_floor_cents.unwrap_or(0))
    .bind(&body.notes)
    .fetch_one(&state.pg)
    .await;

    match result {
        Ok(e) => (StatusCode::CREATED, Json(e)).into_response(),
        Err(e) if e.to_string().contains("unique") => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": "slug already exists"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to create exchange");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateExchangeRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, ExchangeConfig>(
        r#"
        UPDATE exchange_configs SET
            name            = COALESCE($2, name),
            status          = COALESCE($3, status),
            endpoint_url    = COALESCE($4, endpoint_url),
            win_price_macro = COALESCE($5, win_price_macro),
            min_floor_cents = COALESCE($6, min_floor_cents),
            notes           = COALESCE($7, notes)
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.status)
    .bind(&body.endpoint_url)
    .bind(&body.win_price_macro)
    .bind(body.min_floor_cents)
    .bind(&body.notes)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(e)) => Json(e).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to update exchange");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let result = sqlx::query(
        r#"
        SELECT
            exchange,
            date::TEXT,
            impressions,
            spend_cents,
            avg_cpm_cents::BIGINT
        FROM exchange_daily_stats
        ORDER BY date DESC, impressions DESC
        LIMIT 90
        "#,
    )
    .fetch_all(&state.pg)
    .await;

    match result {
        Ok(rows) => {
            use sqlx::Row;
            let data: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "exchange":      r.try_get::<String,_>("exchange").unwrap_or_default(),
                        "date":          r.try_get::<String,_>("date").unwrap_or_default(),
                        "impressions":   r.try_get::<i64,_>("impressions").unwrap_or(0),
                        "spend_cents":   r.try_get::<i64,_>("spend_cents").unwrap_or(0),
                        "avg_cpm_cents": r.try_get::<i64,_>("avg_cpm_cents").unwrap_or(0),
                    })
                })
                .collect();
            Json(data).into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to get exchange stats");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
