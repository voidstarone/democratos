//! Serve with governance writes running directly against `services`.

use std::sync::Arc;

use app::{LocalWrites, Services, SessionSigner};

use crate::serve::serve;

/// Serve with governance writes running directly against `services` (single-box
/// / no federation). Convenience over [`serve`].
pub async fn serve_local(
    services: Services,
    session: SessionSigner,
    addr: &str,
    dev_mode: bool,
    secure_cookies: bool,
    dev_unlock_secret: Option<Arc<str>>,
) -> std::io::Result<()> {
    let writes = Arc::new(LocalWrites::new(services.clone()));
    serve(
        services,
        writes,
        session,
        addr,
        dev_mode,
        secure_cookies,
        dev_unlock_secret,
    )
    .await
}
