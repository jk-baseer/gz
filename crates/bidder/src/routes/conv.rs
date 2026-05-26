/// Conversion tracking pixel.
///
/// Advertisers embed this in their thank-you page:
///   <img src="https://bidder.gz-ads.com/conv/{campaign_id}?value=49.99" width="1" height="1"/>
///
/// The ?value= parameter is the conversion value in USD (e.g. order total).
/// We store it as cents (rounded).
use std::sync::Arc;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct ConvQuery {
    /// Conversion value in USD (e.g. "49.99"). Defaults to 0 if omitted.
    value: Option<f64>,
}

// 1×1 transparent GIF — same pixel as imp.rs
const PIXEL: &[u8] = &[
    0x47, 0x49, 0x46, 0x38, 0x39, 0x61, 0x01, 0x00,
    0x01, 0x00, 0x80, 0x00, 0x00, 0xff, 0xff, 0xff,
    0x00, 0x00, 0x00, 0x21, 0xf9, 0x04, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00,
    0x01, 0x00, 0x01, 0x00, 0x00, 0x02, 0x02, 0x44,
    0x01, 0x00, 0x3b,
];

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Path(campaign_id): Path<Uuid>,
    Query(q): Query<ConvQuery>,
) -> impl IntoResponse {
    let value_cents = q
        .value
        .map(|v| (v * 100.0).round() as i64)
        .unwrap_or(0);

    debug!(campaign_id = %campaign_id, value_cents, "conversion pixel");

    let result = sqlx::query(
        r#"
        INSERT INTO conversion_events (campaign_id, value_cents)
        VALUES ($1, $2)
        "#,
    )
    .bind(campaign_id)
    .bind(value_cents)
    .execute(&state.pg)
    .await;

    if let Err(e) = result {
        warn!(error = %e, "failed to record conversion");
    }

    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/gif"),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
            (header::PRAGMA, "no-cache"),
        ],
        PIXEL,
    )
        .into_response()
}
