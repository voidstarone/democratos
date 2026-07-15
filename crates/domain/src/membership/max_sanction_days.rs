//! The absolute ceiling on any sanction.

/// The longest a sanction may ever last: **18 years** (the length a Swedish life
/// sentence is typically commuted to), expressed in days. This is a HARD platform
/// cap — no ban, community rule, or jury verdict may exceed it, so a permanent ban
/// is structurally impossible. Everyone, however difficult, eventually returns to
/// the public debate. Every code path that applies a sanction routes through
/// [`Membership::sanction_for`](crate::Membership::sanction_for), which clamps to
/// this value.
pub const MAX_SANCTION_DAYS: u32 = 18 * 365;
