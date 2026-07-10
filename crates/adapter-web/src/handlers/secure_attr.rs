//! The `; Secure` cookie attribute helper.

/// The `; Secure` cookie attribute when TLS is in play, else nothing. Kept in one
/// place so every `Set-Cookie` the app emits is consistent.
pub(crate) fn secure_attr(secure: bool) -> &'static str {
    if secure {
        "; Secure"
    } else {
        ""
    }
}
