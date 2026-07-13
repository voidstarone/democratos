//! The canonical bytes a trusted-issuer certificate signs.

/// The canonical bytes an [`IssuerCert`](super::issuer_cert::IssuerCert) signs — a
/// fixed, unambiguous layout so the federation root's signature covers every field
/// and no re-ordering can change its meaning. `node` is the issuer node being
/// certified; `epoch` lets a later grant supersede an earlier one (rotation).
pub(super) fn issuer_cert_body(node: u16, epoch: u64) -> String {
    format!("democratos:issuer-cert:v1;node:{node};epoch:{epoch}")
}
