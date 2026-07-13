//! A notifier that records what it was asked to send — for tests.

use std::sync::Mutex;

use app::{Notifier, NotifyError};
use async_trait::async_trait;

/// Captures each `(to_email, accept_url)` it is handed instead of delivering it,
/// so a test can assert an approval actually tried to email the right link. Never
/// fails.
#[derive(Default)]
pub struct RecordingNotifier {
    sent: Mutex<Vec<(String, String)>>,
}

impl RecordingNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every `(to_email, accept_url)` captured so far, in order.
    pub fn sent(&self) -> Vec<(String, String)> {
        self.sent.lock().expect("recording notifier lock").clone()
    }
}

#[async_trait]
impl Notifier for RecordingNotifier {
    async fn notify_invite_approved(
        &self,
        to_email: &str,
        accept_url: &str,
    ) -> Result<(), NotifyError> {
        self.sent
            .lock()
            .expect("recording notifier lock")
            .push((to_email.to_string(), accept_url.to_string()));
        Ok(())
    }
}
