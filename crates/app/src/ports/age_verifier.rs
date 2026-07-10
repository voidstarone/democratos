//! Performs (or stands in for) real age verification.

use async_trait::async_trait;

use domain::UserId;

use crate::Result;

/// Performs (or stands in for) real age verification — an external, jurisdiction-
/// specific concern (ID checks, AV providers like Yoti, card checks). A port so a
/// deployment plugs in its provider; the dev stub auto-approves. The deployment
/// decides *whether* verification is required; this only performs it.
#[async_trait]
pub trait AgeVerifier: Send + Sync {
    /// Verify `user`; return whether they are now age-verified.
    async fn verify(&self, user: UserId) -> Result<bool>;
}
