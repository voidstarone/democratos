use crate::http::command_router::command_router;
use crate::http::command_state::CommandState;
use crate::http::feed_router::feed_router;
use crate::http::feed_state::FeedState;
use crate::http::ingest_router::ingest_router;
use crate::http::ingest_state::IngestState;

/// Serve the change feed, the command endpoint (forwarded writes), and the
/// synchronous-ingest endpoint (standby side of quorum votes) on one node-only
/// address. Every node needs all three: it serves its own feed, executes writes
/// forwarded to it, and applies vote events pushed to it when it is a standby.
pub async fn serve_federation(
    feed: FeedState,
    command: CommandState,
    ingest: IngestState,
    addr: &str,
) -> std::io::Result<()> {
    let router = feed_router(feed)
        .merge(command_router(command))
        .merge(ingest_router(ingest));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await
}
