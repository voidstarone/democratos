//! Whether a review case is still gathering votes or has resolved.

use serde::{Deserialize, Serialize};

use crate::sensitive::sensitive_tag::SensitiveTag;

/// The lifecycle of a sensitive-content review case.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SensitiveCaseStatus {
    /// Still gathering reviewer classifications; the content stays hidden.
    Open,
    /// The quorum was reached and the plurality tag decided the outcome.
    Resolved(SensitiveTag),
}
