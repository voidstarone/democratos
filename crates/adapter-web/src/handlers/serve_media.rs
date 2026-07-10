//! Serve an uploaded media file by its storage key.

use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use crate::AppState;

/// Serve an uploaded media file by its storage key (local store only — a CDN
/// adapter serves its own URLs and this route simply goes unused).
pub async fn serve_media(State(state): State<AppState>, Path(key): Path<String>) -> Response {
    match state.services.media.get(&key).await {
        Ok(Some((content_type, bytes))) => (
            [
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_str(&content_type)
                        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
                ),
                // Never let a browser MIME-sniff stored bytes into a different,
                // possibly executable, type than the one we serve them under.
                (
                    header::X_CONTENT_TYPE_OPTIONS,
                    HeaderValue::from_static("nosniff"),
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
