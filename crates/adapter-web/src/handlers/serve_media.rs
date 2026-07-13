//! Serve an uploaded media file by its storage key.

use axum::{
    extract::{Path, State},
    http::{header, HeaderName, HeaderValue, StatusCode},
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
                // Serve as a displayed resource, never a top-level document, and —
                // belt-and-braces on top of the sanitizer and `nosniff` — forbid any
                // script/plugin execution should a byte sequence ever be coaxed into
                // being treated as active content.
                (
                    header::CONTENT_DISPOSITION,
                    HeaderValue::from_static("inline"),
                ),
                (
                    header::CONTENT_SECURITY_POLICY,
                    HeaderValue::from_static("default-src 'none'; sandbox; frame-ancestors 'none'"),
                ),
                // Media keys are content-addressed (a SHA-256 of the bytes), so a
                // key's content never changes — cache it hard. This is also what a
                // CDN in front of this route keys on.
                (
                    header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ),
                // Let a separate CDN service on another origin read the bytes, but
                // no arbitrary cross-site embedder beyond that.
                (
                    HeaderName::from_static("cross-origin-resource-policy"),
                    HeaderValue::from_static("same-site"),
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
