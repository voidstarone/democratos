//! The canonical bytes a home binding signs.

/// The canonical bytes a home binding signs — a fixed, unambiguous layout so the
/// signature covers every field and no re-ordering can change its meaning.
pub(super) fn binding_body(demos: u64, home_node: u16, allowed_failover: &[u16], epoch: u64) -> String {
    let failover = allowed_failover
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("democratos:home-binding;demos:{demos};home:{home_node};failover:{failover};epoch:{epoch}")
}
