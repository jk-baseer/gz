/// VAST 4.0 document generator for video creatives.
/// The bid markup for video impressions points to GET /vast/:token.
/// Exchanges fetch this URL and parse the VAST XML to play the video.
use std::sync::Arc;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
};
use tracing::warn;

use bidder::token;
use crate::state::AppState;

pub async fn handle(
    State(state): State<Arc<AppState>>,
    Path(token_str): Path<String>,
) -> impl IntoResponse {
    let tok = match token::decode(&token_str) {
        Ok(t) => t,
        Err(e) => {
            warn!(error = %e, "invalid VAST token");
            return (StatusCode::BAD_REQUEST, "invalid token").into_response();
        }
    };

    // Look up the video creative from the in-memory index
    let campaigns = state.index.get_all().await;
    let creative = campaigns
        .iter()
        .find(|c| c.id == tok.campaign_id)
        .and_then(|c| c.creatives.iter().find(|cr| cr.id == tok.creative_id));

    let (asset_url, duration, w, h) = match creative {
        Some(cr) => (
            cr.asset_url.clone(),
            "00:00:30",
            cr.width.unwrap_or(1280),
            cr.height.unwrap_or(720),
        ),
        None => {
            warn!(campaign_id = %tok.campaign_id, "creative not found for VAST");
            return StatusCode::NOT_FOUND.into_response();
        }
    };

    let imp_url = format!("https://{}/imp/{token_str}", state.cfg.public_hostname);
    let click_url = format!("https://{}/click/{token_str}", state.cfg.public_hostname);

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<VAST version="4.0" xmlns="http://www.iab.com/VAST">
  <Ad id="{bid_id}">
    <InLine>
      <AdSystem version="1.0">GZ Ads</AdSystem>
      <AdTitle>GZ Ad</AdTitle>
      <Impression id="1"><![CDATA[{imp_url}]]></Impression>
      <Creatives>
        <Creative id="{creative_id}" sequence="1">
          <Linear>
            <Duration>{duration}</Duration>
            <TrackingEvents/>
            <VideoClicks>
              <ClickThrough id="1"><![CDATA[{click_url}]]></ClickThrough>
            </VideoClicks>
            <MediaFiles>
              <MediaFile id="1" delivery="progressive" type="video/mp4"
                         width="{w}" height="{h}" bitrate="2000"
                         scalable="true" maintainAspectRatio="true">
                <![CDATA[{asset_url}]]>
              </MediaFile>
            </MediaFiles>
          </Linear>
        </Creative>
      </Creatives>
    </InLine>
  </Ad>
</VAST>"#,
        bid_id      = tok.bid_id,
        creative_id = tok.creative_id,
    );

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        xml,
    )
        .into_response()
}
