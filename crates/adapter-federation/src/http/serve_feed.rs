use crate::http::feed_router::feed_router;
use crate::http::feed_state::FeedState;

/// Serve the feed on `addr` until the process exits.
pub async fn serve_feed(state: FeedState, addr: &str) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, feed_router(state)).await
}
