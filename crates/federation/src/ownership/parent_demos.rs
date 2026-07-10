//! Resolves a ballot's community from its parent row on the local replica.

use async_trait::async_trait;

use crate::{ParentKind, RegistryError};

/// Resolves a ballot's community from its parent row on the local replica — the
/// one piece of authorization that needs data access, so it is injected into the
/// pure [`authorize`](crate::authorize) rather than baked in. Implemented by the
/// store-aware layer.
#[async_trait]
pub trait ParentDemos: Send + Sync {
    /// The community of the `kind` row with `id`, or `None` if that parent isn't
    /// present locally yet (the ordered feed delivers parents before their
    /// ballots, so a miss just means "retry later").
    async fn parent_demos(&self, kind: ParentKind, id: u64) -> Result<Option<u64>, RegistryError>;
}
