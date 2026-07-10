//! The view-time NSFW visibility gate.

use serde::{Deserialize, Serialize};

/// How an NSFW-flagged post should be presented to a particular viewer under a
/// particular deployment policy. Computed purely so the rule is one auditable
/// place; the presentation layer renders it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Visibility {
    /// Not NSFW — show normally.
    Visible,
    /// NSFW — show behind a content warning the viewer may reveal.
    Blurred,
    /// NSFW and age verification is required but the viewer is unverified —
    /// withhold until they verify.
    Gated,
}

/// Decide how to show a post.
///
/// * Not flagged → [`Visibility::Visible`].
/// * Flagged, and either age verification is off *or* the viewer is verified →
///   [`Visibility::Blurred`] (content warning, revealable).
/// * Flagged, age verification required, viewer unverified → [`Visibility::Gated`].
///
/// `age_verification_required` is the deployment toggle (off in most countries,
/// on where the law — e.g. the UK — demands it).
pub fn visibility(
    is_nsfw: bool,
    is_viewer_age_verified: bool,
    requires_age_verification: bool,
) -> Visibility {
    if !is_nsfw {
        Visibility::Visible
    } else if requires_age_verification && !is_viewer_age_verified {
        Visibility::Gated
    } else {
        Visibility::Blurred
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visibility_gate() {
        // Safe content is always visible.
        assert_eq!(visibility(false, false, true), Visibility::Visible);
        // NSFW with the toggle off: blurred, revealable, regardless of verification.
        assert_eq!(visibility(true, false, false), Visibility::Blurred);
        // NSFW, toggle on, unverified viewer: gated.
        assert_eq!(visibility(true, false, true), Visibility::Gated);
        // NSFW, toggle on, verified viewer: blurred (may reveal).
        assert_eq!(visibility(true, true, true), Visibility::Blurred);
    }
}
