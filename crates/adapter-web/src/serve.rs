//! Bind an address and serve the application over HTTP.

use std::net::SocketAddr;
use std::sync::Arc;

use app::{GovernanceWrites, Services, SessionSigner};

use crate::router::router;

/// Bind `addr` and serve the application over HTTP until the process stops.
/// `writes` is the governance-write gateway; `dev_mode` turns on the `/dev`
/// account-switcher endpoints.
#[allow(clippy::too_many_arguments)]
pub async fn serve(
    services: Services,
    writes: Arc<dyn GovernanceWrites>,
    session: SessionSigner,
    addr: &str,
    dev_mode: bool,
    secure_cookies: bool,
    dev_unlock_secret: Option<Arc<str>>,
) -> std::io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("Democratos listening on http://{addr}");
    if dev_mode {
        println!(
            "dev account switcher enabled{}",
            if dev_unlock_secret.is_some() {
                " (unlock requires the configured secret)"
            } else {
                ""
            }
        );
    }
    // `into_make_service_with_connect_info` surfaces each connection's peer
    // address as `ConnectInfo<SocketAddr>`, which the rate limiter keys on. Without
    // it the limiter would have no trustworthy client identity to bucket by.
    axum::serve(
        listener,
        router(
            services,
            writes,
            session,
            dev_mode,
            secure_cookies,
            dev_unlock_secret,
        )
        .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
}
