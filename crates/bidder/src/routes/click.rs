use std::sync::Arc;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
};
use tracing::{debug, warn};

use bidder::token;
use crate::state::AppState;

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Path(token_str): Path<String>,
) -> impl IntoResponse {
    let tok = match token::decode(&token_str) {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "invalid click token");
            return Redirect::temporary("https://gz-ads.com").into_response();
        }
    };

    debug!(
        campaign_id = %tok.campaign_id,
        exchange    = %tok.exchange,
        "click tracked"
    );

    let result = sqlx::query(
        r#"
        INSERT INTO click_events (campaign_id, creative_id, auction_id, exchange)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(tok.campaign_id)
    .bind(tok.creative_id)
    .bind(&tok.auction_id)
    .bind(&tok.exchange)
    .execute(&state.pg)
    .await;

    if let Err(e) = result {
        warn!(error = %e, "failed to write click event");
    }

    Redirect::temporary(&tok.click_url).into_response()
}
