use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

const TOKEN_TTL_HOURS: i64 = 24;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// Advertiser ID
    pub sub: String,
    pub email: String,
    /// Expiry (Unix timestamp)
    pub exp: i64,
    /// Issued at (Unix timestamp)
    pub iat: i64,
}

pub fn create_token(advertiser_id: Uuid, email: &str, secret: &str) -> anyhow::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: advertiser_id.to_string(),
        email: email.to_owned(),
        iat: now.timestamp(),
        exp: (now + Duration::hours(TOKEN_TTL_HOURS)).timestamp(),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn verify_token(token: &str, secret: &str) -> anyhow::Result<Claims> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

/// Axum extractor — validates the Bearer token and injects caller identity.
/// Returns 401 if missing or invalid.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub advertiser_id: Uuid,
    pub email: String,
}

/// Shared secret stored in request extensions by the auth layer.
/// We pass it through app state via a wrapper.
pub struct JwtSecret(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try Authorization: Bearer <token> header
        let token = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(str::to_owned);

        // Also accept token from cookie (for dashboard UI)
        let token = token.or_else(|| {
            parts
                .headers
                .get("cookie")
                .and_then(|v| v.to_str().ok())
                .and_then(|cookies| {
                    cookies.split(';').find_map(|c| {
                        let c = c.trim();
                        c.strip_prefix("gz_token=").map(str::to_owned)
                    })
                })
        });

        let token = match token {
            Some(t) => t,
            None => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "missing authentication token"})),
                )
                    .into_response())
            }
        };

        // Extract JWT secret from request extensions (populated by AppState)
        let secret = parts
            .extensions
            .get::<JwtSecret>()
            .map(|s| s.0.clone())
            .unwrap_or_default();

        let claims = match verify_token(&token, &secret) {
            Ok(c) => c,
            Err(_) => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "invalid or expired token"})),
                )
                    .into_response())
            }
        };

        let advertiser_id = match Uuid::parse_str(&claims.sub) {
            Ok(id) => id,
            Err(_) => {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "malformed token subject"})),
                )
                    .into_response())
            }
        };

        Ok(AuthUser { advertiser_id, email: claims.email })
    }
}
