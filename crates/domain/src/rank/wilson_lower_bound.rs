//! Reddit's "best" comment ranking: the Wilson score lower bound.

/// z for an 80% confidence interval — Reddit's choice. Higher values are more
/// pessimistic about thinly-voted items.
const Z: f64 = 1.281_551_565_545;

/// The lower bound of the Wilson score confidence interval for a comment's
/// upvote ratio: *given the votes so far, what is the worst plausible true
/// approval rating?* Sort descending.
///
/// This is the other half of "the Reddit ones", and it answers a different
/// question from [`reddit_hot`](crate::reddit_hot). Hot ranks *submissions* by
/// popularity-over-time; this ranks *comments* by confidence, with no time term
/// at all — a good reply from an hour ago should not sink just because it is an
/// hour old.
///
/// Why not plain net score, or a raw ratio:
///
/// * **Net score** (`up - down`) lets a mediocre comment in a huge thread bury
///   an excellent one in a small thread, purely on traffic.
/// * **Raw ratio** (`up / total`) makes 1-of-1 (100%) beat 99-of-100 (99%),
///   which is obviously wrong — one vote is not evidence.
///
/// The lower bound fixes both by penalising uncertainty: 1-of-1 scores ~0.29
/// while 99-of-100 scores ~0.95. As votes accumulate the interval narrows and
/// the score converges on the true ratio, so a comment has to *earn* its
/// confidence.
pub fn wilson_lower_bound(upvotes: u64, downvotes: u64) -> f64 {
    let n = upvotes.saturating_add(downvotes);
    if n == 0 {
        return 0.0;
    }

    let n = n as f64;
    let phat = upvotes as f64 / n;

    let numerator =
        phat + Z * Z / (2.0 * n) - Z * ((phat * (1.0 - phat) + Z * Z / (4.0 * n)) / n).sqrt();
    let denominator = 1.0 + Z * Z / n;

    numerator / denominator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_votes_scores_zero() {
        assert_eq!(wilson_lower_bound(0, 0), 0.0);
    }

    #[test]
    fn a_single_upvote_is_not_treated_as_perfect() {
        let one = wilson_lower_bound(1, 0);
        assert!(one > 0.0 && one < 0.5, "1-of-1 should be modest, got {one}");
    }

    #[test]
    fn many_votes_beat_a_perfect_but_thin_record() {
        // The headline property: 99/100 outranks 1/1, despite the worse ratio.
        let thin_and_perfect = wilson_lower_bound(1, 0);
        let thick_and_excellent = wilson_lower_bound(99, 1);
        assert!(
            thick_and_excellent > thin_and_perfect,
            "{thick_and_excellent} should beat {thin_and_perfect}"
        );
    }

    #[test]
    fn converges_towards_the_true_ratio_as_evidence_accumulates() {
        let small = wilson_lower_bound(8, 2);
        let large = wilson_lower_bound(800, 200);
        assert!(small < large, "more evidence must narrow the penalty");
        assert!(
            (large - 0.8).abs() < 0.05,
            "should approach the 0.8 ratio, got {large}"
        );
    }

    #[test]
    fn more_downvotes_always_score_lower() {
        let clean = wilson_lower_bound(50, 0);
        let mixed = wilson_lower_bound(50, 25);
        let bad = wilson_lower_bound(50, 100);
        assert!(clean > mixed && mixed > bad);
    }

    #[test]
    fn is_bounded_to_zero_one() {
        for (u, d) in [(0, 100), (100, 0), (1, 1), (10_000, 3)] {
            let s = wilson_lower_bound(u, d);
            assert!((0.0..=1.0).contains(&s), "{u}/{d} gave {s}");
        }
    }

    #[test]
    fn all_downvotes_scores_near_zero() {
        assert!(wilson_lower_bound(0, 100) < 0.05);
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        let s = wilson_lower_bound(u64::MAX, u64::MAX);
        assert!(s.is_finite());
    }
}
