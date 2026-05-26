/// Stripe wallet top-up routes.
/// POST /billing/checkout  — create a Stripe Checkout session; returns {url}
/// POST /billing/webhook   — Stripe sends checkout.session.completed events here
use std::sync::Arc;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{auth::AuthUser, state::AppState};

#[derive(Deserialize)]
pub struct CheckoutRequest {
    /// Amount in US cents to add to wallet (min 1000 = $10)
    pub amount_cents: i64,
    /// URL to redirect after successful payment
    pub success_url: String,
    /// URL to redirect on cancel
    pub cancel_url: String,
}

#[derive(Serialize)]
pub struct CheckoutResponse {
    pub session_id: String,
    pub url: String,
}

pub async fn checkout(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Json(body): Json<CheckoutRequest>,
) -> impl IntoResponse {
    if body.amount_cents < 1000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "minimum top-up is $10.00 (1000 cents)"})),
        )
            .into_response();
    }

    if state.cfg.stripe_secret_key.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": "Stripe not configured"})),
        )
            .into_response();
    }

    // Build the Stripe Checkout session via the REST API (form-encoded)
    let params = [
        ("payment_method_types[]", "card".to_string()),
        ("mode", "payment".to_string()),
        ("line_items[0][price_data][currency]", "usd".to_string()),
        (
            "line_items[0][price_data][product_data][name]",
            format!("GZ Ads Wallet Top-Up — ${:.2}", body.amount_cents as f64 / 100.0),
        ),
        (
            "line_items[0][price_data][unit_amount]",
            body.amount_cents.to_string(),
        ),
        ("line_items[0][quantity]", "1".to_string()),
        ("success_url", body.success_url.clone()),
        ("cancel_url", body.cancel_url.clone()),
        // Embed advertiser ID so we can credit the right wallet on webhook
        ("metadata[advertiser_id]", user.advertiser_id.to_string()),
        ("metadata[amount_cents]", body.amount_cents.to_string()),
    ];

    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(&state.cfg.stripe_secret_key, None::<&str>)
        .form(&params)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "stripe request failed");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    if !resp.status().is_success() {
        let body_text = resp.text().await.unwrap_or_default();
        tracing::error!(stripe_error = %body_text, "stripe returned error");
        return StatusCode::BAD_GATEWAY.into_response();
    }

    let session: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "failed to parse stripe response");
            return StatusCode::BAD_GATEWAY.into_response();
        }
    };

    let session_id = session["id"].as_str().unwrap_or("").to_owned();
    let url = session["url"].as_str().unwrap_or("").to_owned();

    Json(CheckoutResponse { session_id, url }).into_response()
}

pub async fn webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Verify Stripe-Signature header
    let sig = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_owned(),
        None => return StatusCode::BAD_REQUEST.into_response(),
    };

    if !state.cfg.stripe_webhook_secret.is_empty()
        && !verify_stripe_signature(&body, &sig, &state.cfg.stripe_webhook_secret)
    {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };

    if event["type"].as_str() != Some("checkout.session.completed") {
        return StatusCode::OK.into_response();
    }

    let meta = &event["data"]["object"]["metadata"];
    let advertiser_id = match meta["advertiser_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
    {
        Some(id) => id,
        None => return StatusCode::BAD_REQUEST.into_response(),
    };
    let amount_cents: i64 = meta["amount_cents"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if amount_cents > 0 {
        let _ = sqlx::query(
            r#"
            UPDATE advertisers
            SET wallet_balance_cents = wallet_balance_cents + $2
            WHERE id = $1
            "#,
        )
        .bind(advertiser_id)
        .bind(amount_cents)
        .execute(&state.pg)
        .await;

        tracing::info!(
            advertiser_id = %advertiser_id,
            amount_cents,
            "wallet credited via Stripe"
        );
    }

    StatusCode::OK.into_response()
}

/// Verifies the Stripe-Signature header using HMAC-SHA256.
/// Returns true if the signature matches.
fn verify_stripe_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;

    let mut parts = std::collections::HashMap::new();
    for pair in signature.split(',') {
        if let Some((k, v)) = pair.split_once('=') {
            parts.insert(k, v);
        }
    }

    let timestamp = match parts.get("t") {
        Some(t) => *t,
        None => return false,
    };
    let expected = match parts.get("v1") {
        Some(v) => *v,
        None => return false,
    };

    let payload_str = match std::str::from_utf8(payload) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let signed = format!("{timestamp}.{payload_str}");

    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(signed.as_bytes());
    let result = mac.finalize().into_bytes();
    let computed: String = result.iter().map(|b| format!("{b:02x}")).collect();

    computed == expected
}
