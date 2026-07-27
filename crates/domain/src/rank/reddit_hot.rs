//! Reddit's "hot" ranking: logarithmic vote weight plus a linear time term.

use crate::Timestamp;

/// Seconds of age worth one order of magnitude of votes. Reddit's value: a post
/// must gain 10x the votes to hold its place against something ~12.5 hours newer.
const AGE_PER_ORDER_OF_MAGNITUDE: f64 = 45_000.0;

/// Origin the age term is measured from. This is a pure constant offset applied
/// identically to every post, so its value **cannot change the ordering** — it
/// exists only to keep the numbers small and readable. 2020-01-01T00:00:00Z.
const EPOCH: i64 = 1_577_836_800;

/// Reddit's `hot` score. Two terms, deliberately unlike
/// [`postmill_ranking`](crate::postmill_ranking):
///
/// * **Logarithmic votes.** The first 10 upvotes move a post as much as the next
///   90, and as much again as the next 900. Early votes dominate.
/// * **Linear time.** Every [`AGE_PER_ORDER_OF_MAGNITUDE`] seconds of newness is
///   worth exactly one order of magnitude of votes, forever. Nothing is capped,
///   so age always eventually wins.
///
/// The trade against Postmill is worth stating plainly, because it decides what
/// kind of front page you get:
///
/// * Reddit's log curve means a post needs *exponentially* more support to keep
///   climbing, so a mega-thread saturates rather than running away — but it also
///   means the first handful of voters have outsized power to set a ranking,
///   which on a small site is a real capture risk.
/// * **Comments are not an input at all.** This ranks approval, not
///   conversation. A 500-reply argument and a silently upvoted link rank
///   identically. That is the specific thing
///   [`postmill_ranking`](crate::postmill_ranking) sets out to fix.
///
/// Returns `f64`, so sort with `f64::total_cmp` rather than a naive comparator.
pub fn reddit_hot(net_score: i64, created_at: Timestamp) -> f64 {
    let order = (net_score.unsigned_abs().max(1) as f64).log10();

    let sign = match net_score.signum() {
        1 => 1.0,
        -1 => -1.0,
        _ => 0.0,
    };

    let seconds = created_at.0.saturating_sub(EPOCH) as f64;

    sign * order + seconds / AGE_PER_ORDER_OF_MAGNITUDE
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A day after the epoch, so the age term is a tidy non-zero number.
    const T: Timestamp = Timestamp(EPOCH + 86_400);

    #[test]
    fn zero_score_contributes_no_vote_term() {
        let expected = 86_400.0 / AGE_PER_ORDER_OF_MAGNITUDE;
        assert!((reddit_hot(0, T) - expected).abs() < 1e-9);
    }

    #[test]
    fn votes_are_logarithmic_not_linear() {
        let base = reddit_hot(0, T);
        let ten = reddit_hot(10, T) - base;
        let hundred = reddit_hot(100, T) - base;
        // 100 votes is worth twice 10 votes, not ten times.
        assert!((ten - 1.0).abs() < 1e-9, "log10(10) == 1");
        assert!((hundred - 2.0).abs() < 1e-9, "log10(100) == 2");
    }

    #[test]
    fn the_first_ten_votes_matter_as_much_as_the_next_ninety() {
        let first_ten = reddit_hot(10, T) - reddit_hot(1, T);
        let next_ninety = reddit_hot(100, T) - reddit_hot(10, T);
        assert!((first_ten - next_ninety).abs() < 1e-9);
    }

    #[test]
    fn downvotes_push_below_an_unvoted_post() {
        assert!(reddit_hot(-10, T) < reddit_hot(0, T));
        assert!(reddit_hot(-100, T) < reddit_hot(-10, T));
    }

    #[test]
    fn newer_wins_when_scores_match() {
        let older = reddit_hot(50, Timestamp(EPOCH));
        let newer = reddit_hot(50, Timestamp(EPOCH + 45_000));
        assert!(newer > older);
        // ...and that gap is worth exactly one order of magnitude of votes.
        assert!((newer - older - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ten_times_the_votes_offsets_one_age_step() {
        let old_popular = reddit_hot(100, Timestamp(EPOCH));
        let new_quiet = reddit_hot(10, Timestamp(EPOCH + 45_000));
        assert!((old_popular - new_quiet).abs() < 1e-9);
    }

    #[test]
    fn comments_are_not_an_input() {
        // Documents the contrast with postmill_ranking: this signature has no
        // comment term at all, so a thread and a silent link rank identically.
        assert_eq!(reddit_hot(10, T), reddit_hot(10, T));
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let _ = reddit_hot(i64::MAX, Timestamp(i64::MIN));
        let _ = reddit_hot(i64::MIN, Timestamp(i64::MAX));
    }
}
