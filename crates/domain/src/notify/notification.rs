//! A single in-app notification for one recipient.

use serde::{Deserialize, Serialize};

use crate::{NotificationId, NotificationKind, Timestamp, UserId};

/// A single in-app notification addressed to one member. Notifications are
/// node-local presentation state — generated when the triggering content or trial
/// is created on this node — and are never replicated across the federation.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: NotificationId,
    /// Who this notification is for.
    pub recipient: UserId,
    pub kind: NotificationKind,
    pub created_at: Timestamp,
    /// Whether the recipient has seen it (cleared when they open their
    /// notifications). The unread badge counts the `!seen` ones.
    #[serde(default)]
    pub seen: bool,
}

impl Notification {
    pub fn new(
        id: NotificationId,
        recipient: UserId,
        kind: NotificationKind,
        created_at: Timestamp,
    ) -> Self {
        Self {
            id,
            recipient,
            kind,
            created_at,
            seen: false,
        }
    }
}
