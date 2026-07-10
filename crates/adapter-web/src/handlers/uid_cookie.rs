//! Build the signed session cookie recording which user a browser acts as.

use crate::handlers::secure_attr::secure_attr;
use crate::handlers::session_ttl_seconds::SESSION_TTL_SECONDS;

/// Build the session cookie that records which user the browser is acting as.
/// The value is *signed* by [`app::SessionSigner`] over both the id and an
/// absolute expiry (`now_unix + `[`SESSION_TTL_SECONDS`]`), so it can't be forged
/// nor its lifetime extended by the client. `Max-Age` mirrors the signed expiry
/// so the browser also stops sending it once it lapses.
pub(crate) fn uid_cookie(
    session: &app::SessionSigner,
    id: u64,
    now_unix: i64,
    secure: bool,
) -> String {
    let expires_at = now_unix.saturating_add(SESSION_TTL_SECONDS);
    format!(
        "uid={}; Path=/; HttpOnly; SameSite=Lax; Max-Age={}{}",
        session.sign(id, expires_at),
        SESSION_TTL_SECONDS,
        secure_attr(secure)
    )
}
