//! In-app notifications: a member is pinged when they are named (`@handle`) in
//! content or summoned to a jury. Node-local presentation state, opt-in per kind.

pub mod mentions;
pub mod notification;
pub mod notification_kind;
