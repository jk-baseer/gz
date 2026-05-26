use std::sync::Arc;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{auth, models::Advertiser, state::AppState};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub company_name: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub token: String,
    pub advertiser_id: Uuid,
    pub email: String,
}

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RegisterRequest>,
) -> impl IntoResponse {
    if body.password.len() < 8 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "password must be at least 8 characters"})),
        )
            .into_response();
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = match Argon2::default().hash_password(body.password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let result = sqlx::query_as::<_, Advertiser>(
        r#"
        INSERT INTO advertisers (email, company_name, password_hash)
        VALUES ($1, $2, $3)
        RETURNING *
        "#,
    )
    .bind(&body.email)
    .bind(&body.company_name)
    .bind(&hash)
    .fetch_one(&state.pg)
    .await;

    match result {
        Ok(a) => {
            let token = match auth::create_token(a.id, &a.email, &state.cfg.jwt_secret) {
                Ok(t) => t,
                Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            };
            (
                StatusCode::CREATED,
                Json(TokenResponse { token, advertiser_id: a.id, email: a.email }),
            )
                .into_response()
        }
        Err(e) if e.to_string().contains("unique") => (
            StatusCode::CONFLICT,
            Json(json!({"error": "email already registered"})),
        )
            .into_response(),
        Err(e) => {
            tracing::error!(error = %e, "register failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, Advertiser>(
        "SELECT * FROM advertisers WHERE email = $1"
    )
    .bind(&body.email)
    .fetch_optional(&state.pg)
    .await;

    let advertiser = match row {
        Ok(Some(a)) => a,
        Ok(None) => {
            // Constant-time: still hash to prevent timing attacks
            let _ = Argon2::default().verify_password(b"x", &PasswordHash::new("$argon2id$v=19$m=19456,t=2,p=1$fake").unwrap_or_else(|_| unreachable!()));
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "invalid credentials"})),
            )
                .into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "login db error");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let hash_str = match &advertiser.password_hash {
        Some(h) => h.clone(),
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "account has no password set"})),
            )
                .into_response()
        }
    };

    let parsed_hash = match PasswordHash::new(&hash_str) {
        Ok(h) => h,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if Argon2::default()
        .verify_password(body.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error": "invalid credentials"})),
        )
            .into_response();
    }

    let token = match auth::create_token(advertiser.id, &advertiser.email, &state.cfg.jwt_secret) {
        Ok(t) => t,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Set cookie for dashboard UI + return JSON for API clients
    let cookie = format!(
        "gz_token={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=86400",
        token
    );

    (
        StatusCode::OK,
        [(header::SET_COOKIE, cookie)],
        Json(TokenResponse {
            token,
            advertiser_id: advertiser.id,
            email: advertiser.email,
        }),
    )
        .into_response()
}

pub async fn logout() -> impl IntoResponse {
    let clear = "gz_token=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0";
    (
        StatusCode::OK,
        [(header::SET_COOKIE, clear)],
        Json(json!({"ok": true})),
    )
}
