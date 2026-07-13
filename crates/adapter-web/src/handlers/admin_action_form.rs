//! Form fields shared by the admin review-queue actions (approve / reject /
//! toggle invite-only).

use serde::Deserialize;

/// The secret + CSRF token every review-queue action POST must carry. The secret
/// rides in the form (not just the page URL) so the same subnet-and-secret gate
/// guards the action, not only the page that renders the buttons.
#[derive(Deserialize)]
pub struct AdminActionForm {
    #[serde(default)]
    pub(crate) key: String,
    #[serde(default)]
    pub(crate) csrf_token: String,
    /// The target request id — present on approve/reject, absent on the toggle.
    #[serde(default)]
    pub(crate) id: Option<u64>,
    /// Present on the invite-only toggle form: the desired new state ("on" to
    /// enable). Absent on approve/reject.
    #[serde(default)]
    pub(crate) enabled: Option<String>,
}
