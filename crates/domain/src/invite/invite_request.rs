//! A waitlist entry: someone asking for an account on an invite-only node.

use serde::{Deserialize, Serialize};

use crate::{InviteId, InviteStatus, Timestamp};

/// One entry on the access waitlist. A visitor submits an email (and an optional
/// note); the operator later approves it — issuing a one-time token — or rejects
/// it. Node-local and never federated: the waitlist gates account creation on the
/// node that hosts it, nothing more.
///
/// SECURITY: only the SHA-256 [`token_hash`](Self::token_hash) is ever stored,
/// never the raw token — a leaked store yields no working invite links, exactly
/// as a leaked user table yields no passwords.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct InviteRequest {
    pub id: InviteId,
    /// The requester's email, already normalized (see [`crate::normalize_email`]).
    pub email: String,
    /// An optional short note from the requester ("why I want in").
    #[serde(default)]
    pub note: Option<String>,
    pub status: InviteStatus,
    pub requested_at: Timestamp,
    /// When the operator approved or rejected it; `None` while still pending.
    #[serde(default)]
    pub decided_at: Option<Timestamp>,
    /// SHA-256 (hex) of the one-time invite token. `Some` only once approved.
    #[serde(default)]
    pub token_hash: Option<String>,
    /// When the issued token stops working. `Some` only once approved.
    #[serde(default)]
    pub token_expires_at: Option<Timestamp>,
}

impl InviteRequest {
    /// A fresh, pending request straight off the public form.
    pub fn new(
        id: InviteId,
        email: impl Into<String>,
        note: Option<String>,
        requested_at: Timestamp,
    ) -> Self {
        Self {
            id,
            email: email.into(),
            note,
            status: InviteStatus::Pending,
            requested_at,
            decided_at: None,
            token_hash: None,
            token_expires_at: None,
        }
    }

    /// Approve this request: record the token hash and its expiry. Pure — the
    /// caller decides the token and expiry; the store persists the result.
    pub fn approve(&mut self, token_hash: impl Into<String>, expires_at: Timestamp, at: Timestamp) {
        self.status = InviteStatus::Approved;
        self.token_hash = Some(token_hash.into());
        self.token_expires_at = Some(expires_at);
        self.decided_at = Some(at);
    }

    /// Reject this request. Terminal.
    pub fn reject(&mut self, at: Timestamp) {
        self.status = InviteStatus::Rejected;
        self.decided_at = Some(at);
    }

    /// Mark the invite consumed once the account has been created. Terminal.
    pub fn mark_accepted(&mut self) {
        self.status = InviteStatus::Accepted;
    }

    /// Whether the issued token can still be redeemed: approved (not yet accepted
    /// or rejected) and not past its expiry. A single-use guard — an `Accepted`
    /// invite never redeems again.
    pub fn is_redeemable(&self, now: Timestamp) -> bool {
        self.status == InviteStatus::Approved
            && self.token_expires_at.is_some_and(|exp| now <= exp)
    }
}
