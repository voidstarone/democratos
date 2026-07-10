//! Age verifier that approves everyone — a development stand-in.

use async_trait::async_trait;

use app::{AgeVerifier, Result};
use domain::UserId;

/// Age verifier that approves everyone — a development stand-in. See crate docs.
#[derive(Default)]
pub struct AutoApproveAgeVerifier;

#[async_trait]
impl AgeVerifier for AutoApproveAgeVerifier {
    async fn verify(&self, _user: UserId) -> Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn verifier_approves() {
        assert!(AutoApproveAgeVerifier.verify(UserId(1)).await.unwrap());
    }
}
