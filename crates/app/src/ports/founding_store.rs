//! Persistence for pending founding petitions.

use async_trait::async_trait;

use domain::{FoundingId, FoundingPetition, Timestamp, UserId};

use crate::Result;

/// Pending founding petitions — a demos-in-waiting that needs co-signers before
/// it becomes a real [`Demos`](domain::Demos). Once quorum is reached the
/// use-case founds the demos and [`delete`](Self::delete)s the petition, so this
/// store only ever holds foundings still gathering sign-offs.
#[async_trait]
pub trait FoundingStore: Send + Sync {
    /// Open a petition for `slug`/`name` on behalf of `founder`.
    async fn create(
        &self,
        slug: &str,
        name: &str,
        founder: UserId,
        created_at: Timestamp,
    ) -> Result<FoundingPetition>;
    async fn get(&self, id: FoundingId) -> Result<Option<FoundingPetition>>;
    /// Record `user`'s sign-off, idempotently (a repeat sign-off is a no-op), and
    /// return the updated petition. Errors with [`StoreError::NotFound`](crate::StoreError::NotFound)
    /// if the petition is gone.
    async fn sign(&self, id: FoundingId, user: UserId) -> Result<FoundingPetition>;
    /// Drop a petition once it has been founded (or abandoned).
    async fn delete(&self, id: FoundingId) -> Result<()>;
    /// Every pending petition, newest first. Backs the public "founding" list.
    async fn list(&self) -> Result<Vec<FoundingPetition>>;
}
