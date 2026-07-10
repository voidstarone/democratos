//! A `ParentDemos` that never resolves anything.

use async_trait::async_trait;

use crate::{ParentDemos, ParentKind, RegistryError};

/// A [`ParentDemos`] that never resolves anything — for non-ballot events and
/// tests that don't exercise ballots.
pub struct NoParents;

#[async_trait]
impl ParentDemos for NoParents {
    async fn parent_demos(
        &self,
        _kind: ParentKind,
        _id: u64,
    ) -> Result<Option<u64>, RegistryError> {
        Ok(None)
    }
}
