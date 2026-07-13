//! Decide a case's outcome from its reviewer tags.

use crate::sensitive::review_quorum::REVIEW_QUORUM;
use crate::sensitive::sensitive_tag::SensitiveTag;

/// Decide the winning classification for a set of reviewer tags, or `None` if the
/// [quorum](REVIEW_QUORUM) has not yet been reached.
///
/// The rule is **plurality wins** — the tag with the most votes — with ties broken
/// by [`SensitiveTag::severity`] so a split decision errs toward the more
/// protective (removing) outcome. This is a pure function of the votes cast, so the
/// resolution is deterministic and exhaustively testable.
pub fn tally_tags(votes: &[SensitiveTag]) -> Option<SensitiveTag> {
    if votes.len() < REVIEW_QUORUM {
        return None;
    }
    SensitiveTag::ALL
        .into_iter()
        .map(|tag| (tag, votes.iter().filter(|&&v| v == tag).count()))
        .filter(|&(_, count)| count > 0)
        // Pick the tag with the most votes; on a tie, the more severe one.
        .max_by(|(a, ca), (b, cb)| ca.cmp(cb).then(a.severity().cmp(&b.severity())))
        .map(|(tag, _)| tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use SensitiveTag::*;

    #[test]
    fn no_verdict_before_quorum() {
        assert_eq!(tally_tags(&[Csam, Csam, Csam, Csam]), None); // only 4
    }

    #[test]
    fn clear_plurality_wins() {
        let votes = [Porn, Porn, Porn, Gore, NotSensitive];
        assert_eq!(tally_tags(&votes), Some(Porn));
    }

    #[test]
    fn a_tie_breaks_toward_the_more_severe_tag() {
        // Csam and Porn both have 2; NotSensitive has 1. Csam (more severe) wins.
        let votes = [Csam, Csam, Porn, Porn, NotSensitive];
        assert_eq!(tally_tags(&votes), Some(Csam));
    }

    #[test]
    fn not_sensitive_wins_only_on_a_clear_plurality() {
        let votes = [NotSensitive, NotSensitive, NotSensitive, Porn, Gore];
        assert_eq!(tally_tags(&votes), Some(NotSensitive));
        // But a tie never resolves to restore:
        let tied = [NotSensitive, NotSensitive, Spam, Spam, Porn];
        assert_eq!(tally_tags(&tied), Some(Spam));
    }
}
