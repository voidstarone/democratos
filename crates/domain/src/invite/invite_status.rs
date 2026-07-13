//! Where an invite request is in its lifecycle.

use serde::{Deserialize, Serialize};

/// Where an invite request sits on its path from the public waitlist to a real
/// account.
///
/// The flow is linear: a visitor asks to join ([`Pending`](Self::Pending)); the
/// operator either turns them away ([`Rejected`](Self::Rejected)) or issues a
/// one-time invite link ([`Approved`](Self::Approved)); following that link and
/// finishing sign-up consumes the invite ([`Accepted`](Self::Accepted)). Only a
/// `Pending` request can be approved or rejected, and only an `Approved` one can
/// be accepted — so the token is single-use.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum InviteStatus {
    /// On the waitlist, awaiting the operator's decision.
    Pending,
    /// Approved — a one-time token has been issued and the email sent.
    Approved,
    /// The token was redeemed and the account created. Terminal.
    Accepted,
    /// Turned away by the operator. Terminal.
    Rejected,
}
