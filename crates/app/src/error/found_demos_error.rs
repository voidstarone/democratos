//! The error type of [`Services::found_demos`](crate::Services::found_demos) and
//! its franchise-bar guard.

use thiserror::Error;

use crate::StoreError;

/// Why founding a demos failed. Beyond store failures (including the
/// [`AlreadyExists`](crate::StoreError::AlreadyExists) slug clash, which arrives
/// as [`Store`](FoundDemosError::Store)), a franchise-barred account is
/// [`Rejected`](FoundDemosError::Rejected): a dev/content puppet may take no path
/// to the franchise, and founding enfranchises the founder directly.
#[derive(Debug, Error)]
pub enum FoundDemosError {
    #[error(transparent)]
    Store(#[from] StoreError),

    #[error("{0}")]
    Rejected(String),
}
