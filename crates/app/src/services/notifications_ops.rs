//! Facade delegators for notification use-cases. The logic now lives in
//! [`NotificationService`](super::notification_service::NotificationService);
//! these thin methods keep `services.notifications()` and friends working for
//! call sites not yet migrated off the `Services` aggregator.

use domain::{Notification, UserId};

use crate::Result;

use super::notification_service::NotificationService;
use super::services::Services;

impl Services {
    /// Build the extracted [`NotificationService`] from the ports this aggregator
    /// still holds. Cheap — `Arc` clones only — so delegators construct one per
    /// call rather than storing a field (which would break every `Services { … }`
    /// literal). Removed once all call sites inject `NotificationService` directly.
    pub(super) fn notification_service(&self) -> NotificationService {
        NotificationService::new(
            self.notifications.clone(),
            self.users.clone(),
            self.clock.clone(),
        )
    }

    /// This member's notifications, newest first.
    pub async fn notifications(&self, user: UserId) -> Result<Vec<Notification>> {
        self.notification_service().notifications(user).await
    }

    /// How many unseen notifications this member has — the toolbar badge count.
    pub async fn unread_notification_count(&self, user: UserId) -> Result<u64> {
        self.notification_service()
            .unread_notification_count(user)
            .await
    }

    /// Mark all this member's notifications seen (they opened their list).
    pub async fn mark_notifications_seen(&self, user: UserId) -> Result<()> {
        self.notification_service().mark_notifications_seen(user).await
    }

    /// Save which notification kinds this member wants.
    pub async fn set_alert_prefs(
        &self,
        user: UserId,
        allows_mention_alerts: bool,
        allows_jury_alerts: bool,
        allows_trial_comment_alerts: bool,
    ) -> Result<()> {
        self.notification_service()
            .set_alert_prefs(
                user,
                allows_mention_alerts,
                allows_jury_alerts,
                allows_trial_comment_alerts,
            )
            .await
    }
}
