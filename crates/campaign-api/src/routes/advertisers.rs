use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{
    models::{Advertiser, CreateAdvertiserRequest, TopUpRequest},
    state::AppState,
};

pub async fn create(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateAdvertiserRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Advertiser>(
        "INSERT INTO advertisers (email, company_name) VALUES ($1, $2) RETURNING *",
    )
    .bind(&body.email)
    .bind(&body.company_name)
    .fetch_one(&state.pg)
    .await;

    match result {
        Ok(a) => (StatusCode::CREATED, Json(a)).into_response(),
        Err(e) if e.to_string().contains("unique") => {
            (StatusCode::CONFLICT, "email already registered").into_response()
        }
        Err(e) => {
            tracing::error!(error = %e, "create advertiser failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn get_one(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Advertiser>(
        "SELECT * FROM advertisers WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(a)) => Json(a).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "get advertiser failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn topup(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
    Json(body): Json<TopUpRequest>,
) -> impl IntoResponse {
    if body.amount_cents <= 0 {
        return (StatusCode::BAD_REQUEST, "amount must be positive").into_response();
    }

    let result = sqlx::query_as::<_, Advertiser>(
        r#"
        UPDATE advertisers
        SET wallet_balance_cents = wallet_balance_cents + $2,
            updated_at           = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(body.amount_cents)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(a)) => Json(a).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "topup failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
