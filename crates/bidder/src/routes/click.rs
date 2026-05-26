use axum::{
    extract::Path,
    http::{header, StatusCode},
    response::IntoResponse,
};
use tracing::debug;

pub async fn handle(Path(token): Path<String>) -> impl IntoResponse {
    debug!(token = %token, "click tracked");
    // TODO: decode token to get destination URL and campaign info,
    //       publish click event to Redpanda, then redirect
    (
        StatusCode::FOUND,
        [(header::LOCATION, "/")], // placeholder until token decoding is implemented
    )
        .into_response()
}
