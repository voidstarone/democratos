/// The exact bytes signed: a domain-separated join of every field that must be
/// authenticated, so tampering with any one (including the anti-replay metadata)
/// breaks the signature.
pub(crate) fn signing_payload(node: u16, issued_at: i64, nonce: &str, body: &str) -> String {
    format!("democratos:cmd:v1\n{node}\n{issued_at}\n{nonce}\n{body}")
}
