//! The version + protocol tag prefixing every signed message.

/// Version + protocol tag prefixing every signed message, so a Democratos v1
/// action signature is meaningless in any other context.
pub(crate) const DOMAIN: &str = "democratos:v1";
