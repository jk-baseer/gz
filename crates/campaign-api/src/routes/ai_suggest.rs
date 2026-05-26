use std::sync::Arc;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

#[derive(Deserialize)]
pub struct SuggestRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiSuggestion {
    pub suggested_name: String,
    pub suggested_bid_cpm_cents: i64,
    pub geo_countries: Vec<String>,
    pub device_types: Vec<String>,
    pub os_types: Vec<String>,
    pub languages: Vec<String>,
    pub site_categories: Vec<String>,
    pub domain_allowlist: Vec<String>,
    pub keywords: Vec<String>,
    pub age_min: Option<i32>,
    pub age_max: Option<i32>,
    pub reasoning: String,
}

const SYSTEM_PROMPT: &str = r#"You are a campaign targeting assistant for a programmatic advertising DSP serving GCC/MENA advertisers.

Convert the advertiser's natural-language campaign goal into targeting parameters.

Return ONLY valid JSON — no markdown, no code fences, no explanation outside the JSON:
{
  "suggested_name": "short campaign name (5 words max)",
  "suggested_bid_cpm_cents": 200,
  "geo_countries": [],
  "device_types": [],
  "os_types": [],
  "languages": [],
  "site_categories": [],
  "domain_allowlist": [],
  "keywords": [],
  "age_min": null,
  "age_max": null,
  "reasoning": "one sentence"
}

Rules:
- geo_countries: ISO-3166-1-alpha-3. GCC countries: UAE=ARE, Saudi=SAU, Kuwait=KWT, Qatar=QAT, Bahrain=BHR, Oman=OMN. Other MENA: Egypt=EGY, Jordan=JOR, Iraq=IRQ, Lebanon=LBN, Morocco=MAR, Tunisia=TUN. Empty array = all countries.
- device_types: subset of ["mobile","desktop","tablet"]. Empty = all.
- os_types: subset of ["ios","android","windows","macos"]. Empty = all.
- languages: BCP-47 codes. Arabic=ar, English=en. Empty = all.
- site_categories: IAB taxonomy. IAB1=Arts, IAB2=Automotive, IAB3=Business, IAB4=Careers, IAB5=Education, IAB6=Family, IAB7=Health/Fitness, IAB8=Food, IAB9=Hobbies, IAB10=Home, IAB11=Law/Government, IAB12=News, IAB13=Finance, IAB14=Society, IAB15=Science, IAB16=Pets, IAB17=Sports, IAB18=Fashion, IAB19=Technology, IAB20=Travel, IAB21=Real Estate, IAB22=Shopping. Empty = all.
- domain_allowlist: only set if advertiser names specific industry sites (e.g. fleetowner.com). Usually leave empty.
- keywords: 4–8 relevant terms that appear in industry content (e.g. "fleet", "logistics", "ERP"). Empty = all.
- suggested_bid_cpm_cents: B2B/enterprise 300–500, consumer brand 150–250, awareness/broad 100–200.
- age_min/age_max: only set if advertiser specifies an age. null means unconstrained.

For B2B campaigns: prefer desktop device type, IAB3/IAB4/IAB19, relevant keywords, and skip OS targeting."#;

pub async fn suggest(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SuggestRequest>,
) -> impl IntoResponse {
    let api_key = &state.cfg.anthropic_api_key;
    if api_key.is_empty() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "ANTHROPIC_API_KEY not configured"})),
        )
            .into_response();
    }

    let payload = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "system": SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": body.prompt}]
    });

    let resp = state
        .http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key.as_str())
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(error = %e, "anthropic request failed");
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": "upstream request failed"})),
            )
                .into_response();
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        tracing::error!(status = %status, body = %body, "anthropic error");
        return (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": format!("AI service error: {status}")})),
        )
            .into_response();
    }

    let anthropic_resp: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "failed to parse anthropic response");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // Extract text from content[0].text
    let text = anthropic_resp["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .trim();

    match serde_json::from_str::<AiSuggestion>(text) {
        Ok(suggestion) => Json(suggestion).into_response(),
        Err(e) => {
            tracing::error!(error = %e, raw = %text, "failed to parse AI JSON");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "AI returned invalid JSON", "raw": text})),
            )
                .into_response()
        }
    }
}
