//! Release handle reservations this node holds that no account backs.

use adapter_store_postgres::PostgresStore;
use app::UserStore;
use domain::NodeId;
use federation::OwnershipRegistry;

/// Reconcile away **orphaned** handle reservations — ones a crash left behind in the
/// reserve→create window, with no account to back them. Releases a handle ONLY when
/// no local account uses it, so a live account's handle is never touched. Best-effort
/// and idempotent: a control-plane hiccup just leaves an orphan for the next startup.
pub async fn reconcile_orphan_handles(
    store: &PostgresStore,
    registry: &dyn OwnershipRegistry,
    node: NodeId,
) {
    let handles = match registry.reserved_handles(node).await {
        Ok(h) => h,
        Err(e) => {
            eprintln!("⚠ federation: could not list handle reservations to reconcile: {e}");
            return;
        }
    };
    let mut freed = 0;
    for handle in handles {
        match store.by_handle(&handle).await {
            Ok(Some(_)) => {} // a real account backs it — keep the reservation
            Ok(None) => {
                if registry.release_handle(&handle, node).await.is_ok() {
                    freed += 1;
                }
            }
            Err(e) => eprintln!("⚠ federation: reconcile lookup for '{handle}' failed: {e}"),
        }
    }
    if freed > 0 {
        eprintln!("federation: released {freed} orphaned handle reservation(s)");
    }
}
