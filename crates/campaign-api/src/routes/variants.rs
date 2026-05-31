use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct GenerateVariantsRequest {
    pub brief: String,
    #[serde(default = "default_n_variants")]
    pub n_variants: u8,
}

fn default_n_variants() -> u8 { 5 }

#[derive(Debug, Serialize, Deserialize)]
struct NativeVariant {
    title_text: Option<String>,
    description: Option<String>,
    cta_text: Option<String>,
    variant_hypothesis: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct BannerVariant {
    html_adm: String,
    variant_hypothesis: Option<String>,
}

const NATIVE_SYSTEM: &str = r#"You are an ad creative copywriter specialising in GCC/MENA markets.
Generate variations of a native ad. Each variant should test a different angle: e.g. urgency, social proof, benefit-led, question-led, or feature-led.
Return ONLY a valid JSON array — no markdown, no code fences:
[
  { "title_text": "...", "description": "...", "cta_text": "...", "variant_hypothesis": "one phrase describing what this variant tests" },
  ...
]
Rules: title_text ≤ 25 words, description ≤ 90 chars, cta_text ≤ 5 words. All copy in English unless brief specifies Arabic."#;

const BANNER_SYSTEM: &str = r#"You are an HTML5 ad creative developer specialising in GCC/MENA markets.
Generate self-contained HTML5 banner ad markup. Each variant must test a different headline, CTA text, or accent colour.
Use {{CLICK_URL}} for the click destination and {{IMP_URL}} for the impression tracker (1×1 pixel).
Return ONLY a valid JSON array — no markdown, no code fences:
[
  { "html_adm": "<full self-contained HTML>", "variant_hypothesis": "one phrase describing what this variant tests" },
  ...
]
Requirements for html_adm:
- Self-contained: inline CSS only, no external stylesheets or scripts
- Include: <a href="{{CLICK_URL}}"> wrapping the creative, and <img src="{{IMP_URL}}" width="1" height="1" style="display:none"> at the end
- The base image should be shown as a background-image or <img> using the provided asset_url
- Clean, mobile-friendly design with a clear CTA button"#;

pub async fn generate(
    State(state): State<Arc<AppState>>,
    Path((campaign_id, creative_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<GenerateVariantsRequest>,
) -> impl IntoResponse {
    let api_key = &state.cfg.anthropic_api_key;
    if api_key.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "ANTHROPIC_API_KEY not configured"})),
        ).into_response();
    }

    let n = body.n_variants.clamp(2, 8);

    // Load base creative
    let row = sqlx::query(
        "SELECT format, width, height, asset_url, title_text, description, cta_text, sponsored_by \
         FROM creatives WHERE id = $1 AND campaign_id = $2"
    )
    .bind(creative_id)
    .bind(campaign_id)
    .fetch_optional(&state.pg)
    .await;

    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!(error = %e, "failed to load base creative");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let format: String = sqlx::Row::try_get(&row, "format").unwrap_or_default();
    let asset_url: String = sqlx::Row::try_get(&row, "asset_url").unwrap_or_default();
    let width: Option<i32> = sqlx::Row::try_get(&row, "width").ok().flatten();
    let height: Option<i32> = sqlx::Row::try_get(&row, "height").ok().flatten();
    let title: Option<String> = sqlx::Row::try_get(&row, "title_text").ok().flatten();
    let desc: Option<String> = sqlx::Row::try_get(&row, "description").ok().flatten();
    let cta: Option<String> = sqlx::Row::try_get(&row, "cta_text").ok().flatten();
    let sponsored: Option<String> = sqlx::Row::try_get(&row, "sponsored_by").ok().flatten();

    let (system_prompt, user_message) = match format.as_str() {
        "native" => {
            let base_info = format!(
                "Base ad — Title: {:?} | Description: {:?} | CTA: {:?} | Sponsored by: {:?}\n\nAdvertiser brief: {}",
                title.as_deref().unwrap_or(""),
                desc.as_deref().unwrap_or(""),
                cta.as_deref().unwrap_or(""),
                sponsored.as_deref().unwrap_or(""),
                body.brief,
            );
            (NATIVE_SYSTEM, format!("Generate {n} variants.\n\n{base_info}"))
        }
        "banner" => {
            let size = match (width, height) {
                (Some(w), Some(h)) => format!("{w}×{h}"),
                _ => "300×250".into(),
            };
            let base_info = format!(
                "Base ad — Size: {size}px | Asset URL: {asset_url}\n\nAdvertiser brief: {}",
                body.brief
            );
            (BANNER_SYSTEM, format!("Generate {n} variants.\n\n{base_info}"))
        }
        other => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({"error": format!("variant generation not supported for format '{other}'")})),
            ).into_response();
        }
    };

    // Call Claude Haiku
    let payload = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 4096,
        "system": system_prompt,
        "messages": [{"role": "user", "content": user_message}]
    });

    let resp = state.http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key.as_str())
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await;

    let resp = match resp {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            let status = r.status();
            let text = r.text().await.unwrap_or_default();
            tracing::error!(status = %status, body = %text, "anthropic error");
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "AI service error"}))).into_response();
        }
        Err(e) => {
            tracing::error!(error = %e, "anthropic request failed");
            return (StatusCode::BAD_GATEWAY, Json(serde_json::json!({"error": "upstream request failed"}))).into_response();
        }
    };

    let anthropic_resp: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "failed to parse anthropic response");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let text = anthropic_resp["content"][0]["text"].as_str().unwrap_or("").trim();

    // Insert variants into creatives table
    let mut created: Vec<serde_json::Value> = vec![];

    match format.as_str() {
        "native" => {
            let variants: Vec<NativeVariant> = match serde_json::from_str(text) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, raw = %text, "failed to parse native variants JSON");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "AI returned invalid JSON", "raw": text}))).into_response();
                }
            };

            for v in variants {
                let id = Uuid::new_v4();
                let result = sqlx::query(
                    r#"INSERT INTO creatives
                       (id, campaign_id, format, asset_url, click_url, status,
                        title_text, description, cta_text, sponsored_by,
                        parent_creative_id, variant_hypothesis)
                       VALUES ($1,$2,$3,$4,$5,'approved',$6,$7,$8,$9,$10,$11)
                       RETURNING id, format, title_text, description, cta_text, variant_hypothesis"#,
                )
                .bind(id)
                .bind(campaign_id)
                .bind("native")
                .bind(&asset_url)
                .bind("") // click_url inherited from campaign
                .bind(v.title_text.as_deref())
                .bind(v.description.as_deref())
                .bind(v.cta_text.as_deref())
                .bind(sponsored.as_deref())
                .bind(creative_id)
                .bind(v.variant_hypothesis.as_deref())
                .fetch_one(&state.pg)
                .await;

                match result {
                    Ok(r) => {
                        let row_id: Uuid = sqlx::Row::try_get(&r, "id").unwrap_or(id);
                        created.push(serde_json::json!({
                            "id": row_id,
                            "format": "native",
                            "title_text": v.title_text,
                            "description": v.description,
                            "cta_text": v.cta_text,
                            "variant_hypothesis": v.variant_hypothesis,
                            "parent_creative_id": creative_id,
                        }));
                    }
                    Err(e) => tracing::error!(error = %e, "failed to insert native variant"),
                }
            }
        }

        "banner" => {
            let variants: Vec<BannerVariant> = match serde_json::from_str(text) {
                Ok(v) => v,
                Err(e) => {
                    tracing::error!(error = %e, raw = %text, "failed to parse banner variants JSON");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "AI returned invalid JSON", "raw": text}))).into_response();
                }
            };

            for v in variants {
                let id = Uuid::new_v4();
                let result = sqlx::query(
                    r#"INSERT INTO creatives
                       (id, campaign_id, format, width, height, asset_url, click_url, status,
                        html_adm, parent_creative_id, variant_hypothesis)
                       VALUES ($1,$2,$3,$4,$5,$6,$7,'approved',$8,$9,$10)
                       RETURNING id, format, width, height, variant_hypothesis"#,
                )
                .bind(id)
                .bind(campaign_id)
                .bind("banner")
                .bind(width)
                .bind(height)
                .bind(&asset_url)
                .bind("") // click_url substituted via {{CLICK_URL}} in html_adm
                .bind(&v.html_adm)
                .bind(creative_id)
                .bind(v.variant_hypothesis.as_deref())
                .fetch_one(&state.pg)
                .await;

                match result {
                    Ok(r) => {
                        let row_id: Uuid = sqlx::Row::try_get(&r, "id").unwrap_or(id);
                        created.push(serde_json::json!({
                            "id": row_id,
                            "format": "banner",
                            "width": width,
                            "height": height,
                            "variant_hypothesis": v.variant_hypothesis,
                            "parent_creative_id": creative_id,
                        }));
                    }
                    Err(e) => tracing::error!(error = %e, "failed to insert banner variant"),
                }
            }
        }

        _ => {}
    }

    Json(serde_json::json!({ "created": created.len(), "variants": created })).into_response()
}
