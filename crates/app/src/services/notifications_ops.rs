

use domain::{
    CommentId,
    mentions, Notification, NotificationKind,
    PostId, UserId,
};


use crate::Result;


use super::services::Services;

impl Services {
    /// This member's notifications, newest first.
    pub async fn notifications(&self, user: UserId) -> Result<Vec<Notification>> {
        self.notifications.list_for(user).await
    }

    /// How many unseen notifications this member has — the toolbar badge count.
    pub async fn unread_notification_count(&self, user: UserId) -> Result<u64> {
        self.notifications.unread_count(user).await
    }

    /// Mark all this member's notifications seen (they opened their list).
    pub async fn mark_notifications_seen(&self, user: UserId) -> Result<()> {
        self.notifications.mark_all_seen(user).await
    }

    /// Save which notification kinds this member wants.
    pub async fn set_alert_prefs(
        &self,
        user: UserId,
        allows_mention_alerts: bool,
        allows_jury_alerts: bool,
        allows_trial_comment_alerts: bool,
    ) -> Result<()> {
        self.users
            .set_alert_prefs(
                user,
                allows_mention_alerts,
                allows_jury_alerts,
                allows_trial_comment_alerts,
            )
            .await
    }

    /// Notify every account named as `@handle` in `text` that `author` mentioned
    /// them — skipping the author themselves, unknown handles, and anyone who has
    /// opted out of mention alerts. Best-effort: a mention notification never
    /// blocks the post/comment it rode in on, so this is called after the write
    /// succeeds and its own errors propagate only as a failed notification.
    pub(super) async fn notify_mentions(
        &self,
        author: UserId,
        text: &str,
        post: PostId,
        comment: Option<CommentId>,
    ) -> Result<()> {
        let now = self.clock.now();
        for handle in mentions(text) {
            if let Some(user) = self.users.by_handle(&handle).await? {
                if user.id != author && user.allows_mention_alerts {
                    self.notifications
                        .push(
                            user.id,
                            NotificationKind::Mention {
                                post,
                                comment,
                                by: author,
                            },
                            now,
                        )
                        .await?;
                }
            }
        }
        Ok(())
    }
}
