//! Serve with governance writes running directly against `services`.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use app::{LocalAuthenticator, LocalMinter, LocalWrites, Services, SessionSigner};
use ipnet::IpNet;

use crate::serve::serve;

/// Serve with governance writes running directly against `services` (single-box
/// / no federation). Convenience over [`serve`].
#[allow(clippy::too_many_arguments)]
pub async fn serve_local(
    services: Services,
    session: SessionSigner,
    addr: &str,
    dev_mode: bool,
    secure_cookies: bool,
    dev_unlock_secret: Option<Arc<str>>,
    invite_only: Arc<AtomicBool>,
    admin_subnets: Arc<[IpNet]>,
    admin_secret: Option<Arc<str>>,
) -> std::io::Result<()> {
    let writes = Arc::new(LocalWrites::new(services.clone()));
    let minter = Arc::new(LocalMinter::new(services.clone()));
    let authenticator = Arc::new(LocalAuthenticator::new(services.clone()));
    serve(
        services,
        writes,
        minter,
        authenticator,
        session,
        addr,
        dev_mode,
        secure_cookies,
        dev_unlock_secret,
        invite_only,
        admin_subnets,
        admin_secret,
    )
    .await
}
