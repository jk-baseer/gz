use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::{models::{Targeting, UpdateTargetingRequest}, state::AppState};

pub async fn get(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Targeting>(
        "SELECT * FROM campaign_targeting WHERE campaign_id = $1"
    )
    .bind(campaign_id)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "get targeting failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn update(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    Json(body): Json<UpdateTargetingRequest>,
) -> impl IntoResponse {
    let result = sqlx::query_as::<_, Targeting>(
        r#"
        INSERT INTO campaign_targeting (
            campaign_id, geo_countries, device_types, os_types,
            site_categories, languages, hours_of_day, days_of_week,
            domain_allowlist, domain_blocklist, keywords, age_min, age_max
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        ON CONFLICT (campaign_id) DO UPDATE SET
            geo_countries    = COALESCE($2,  campaign_targeting.geo_countries),
            device_types     = COALESCE($3,  campaign_targeting.device_types),
            os_types         = COALESCE($4,  campaign_targeting.os_types),
            site_categories  = COALESCE($5,  campaign_targeting.site_categories),
            languages        = COALESCE($6,  campaign_targeting.languages),
            hours_of_day     = COALESCE($7,  campaign_targeting.hours_of_day),
            days_of_week     = COALESCE($8,  campaign_targeting.days_of_week),
            domain_allowlist = COALESCE($9,  campaign_targeting.domain_allowlist),
            domain_blocklist = COALESCE($10, campaign_targeting.domain_blocklist),
            keywords         = COALESCE($11, campaign_targeting.keywords),
            age_min          = COALESCE($12, campaign_targeting.age_min),
            age_max          = COALESCE($13, campaign_targeting.age_max)
        RETURNING *
        "#,
    )
    .bind(campaign_id)
    .bind(body.geo_countries.as_deref())
    .bind(body.device_types.as_deref())
    .bind(body.os_types.as_deref())
    .bind(body.site_categories.as_deref())
    .bind(body.languages.as_deref())
    .bind(body.hours_of_day.as_deref())
    .bind(body.days_of_week.as_deref())
    .bind(body.domain_allowlist.as_deref())
    .bind(body.domain_blocklist.as_deref())
    .bind(body.keywords.as_deref())
    .bind(body.age_min)
    .bind(body.age_max)
    .fetch_optional(&state.pg)
    .await;

    match result {
        Ok(Some(t)) => Json(t).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "update targeting failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
