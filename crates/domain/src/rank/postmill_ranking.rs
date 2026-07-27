//! Postmill's "hot" ranking — the algorithm Raddle sorts its front page by.

use crate::Timestamp;

/// Seconds of pretend-freshness one net upvote buys.
const NET_SCORE_MULTIPLIER: i64 = 1_800;
/// Seconds one comment buys, on a post the room has not turned against.
/// Deliberately larger than [`NET_SCORE_MULTIPLIER`]: discussion is worth more
/// than passive approval.
const COMMENT_MULTIPLIER: i64 = 5_000;
/// Seconds one comment buys once a post is at or below [`DOWNVOTED_CUTOFF`].
/// Ten times smaller, so an argument under a disliked post cannot carry it.
const COMMENT_DOWNVOTED_MULTIPLIER: i64 = 500;
/// Net score at or below which the comment bonus collapses to
/// [`COMMENT_DOWNVOTED_MULTIPLIER`].
const DOWNVOTED_CUTOFF: i64 = -5;
/// Most a post can be pushed forward: one day.
const MAX_ADVANTAGE: i64 = 86_400;
/// Most a post can be pushed back: half a day.
const MAX_PENALTY: i64 = 43_200;

/// Postmill's `hot` score: **a timestamp, not a rating**. A post is ranked as if
/// it had been posted `advantage` seconds later than it actually was, so sorting
/// descending by this value orders the feed. Because the result is an absolute
/// instant, the whole feed ages uniformly — there is no decay curve to re-run,
/// and a post's rank is fixed the moment its votes and comments are.
///
/// Three properties are the point of the design:
///
/// 1. **A comment outweighs an upvote ~2.8:1** ([`COMMENT_MULTIPLIER`] vs
///    [`NET_SCORE_MULTIPLIER`]). The ceiling is reached at 18 comments but takes
///    48 net upvotes — a post that starts a conversation climbs on a fraction of
///    the audience a post that merely pleases people needs.
/// 2. **The comment bonus collapses 10x below [`DOWNVOTED_CUTOFF`].** This taxes
///    the case where "engagement" and "quality" diverge — a widely disliked post
///    farming a flamewar. Note it is a **tax, not a wall**: a post at -20 still
///    overtakes a modestly-liked one once it passes ~106 replies, and still
///    reaches the cap at 245. It raises the price of ragebait by an order of
///    magnitude; it does not make it unreachable.
/// 3. **Bounded both ways** ([`MAX_ADVANTAGE`] / [`MAX_PENALTY`]). The best post
///    gets a one-day head start and no more, so nothing squats on the front
///    page; the worst is buried half a day, not forever.
///
/// Note it is *linear*, unlike Reddit's logarithmic
/// [`reddit_hot`](crate::reddit_hot) — the caps do the compression that a log
/// otherwise would. On a small forum that matters: no handful of early voters
/// can lock in a ranking, because the 1st and 40th upvote are worth the same.
pub fn postmill_ranking(net_score: i64, comment_count: u64, created_at: Timestamp) -> i64 {
    let net_advantage = net_score.saturating_mul(NET_SCORE_MULTIPLIER);

    let per_comment = if net_score > DOWNVOTED_CUTOFF {
        COMMENT_MULTIPLIER
    } else {
        COMMENT_DOWNVOTED_MULTIPLIER
    };
    let comment_advantage = i64::try_from(comment_count)
        .unwrap_or(i64::MAX)
        .saturating_mul(per_comment);

    let advantage = net_advantage
        .saturating_add(comment_advantage)
        .clamp(-MAX_PENALTY, MAX_ADVANTAGE);

    created_at.0.saturating_add(advantage)
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: Timestamp = Timestamp(1_000_000);

    #[test]
    fn no_votes_or_comments_ranks_at_its_own_timestamp() {
        assert_eq!(postmill_ranking(0, 0, T), T.0);
    }

    #[test]
    fn a_comment_outweighs_an_upvote() {
        let one_comment = postmill_ranking(0, 1, T);
        let one_upvote = postmill_ranking(1, 0, T);
        assert!(
            one_comment > one_upvote,
            "discussion must beat passive approval: {one_comment} vs {one_upvote}"
        );
        // 5000s vs 1800s — the ratio is the design, so pin it.
        assert_eq!(one_comment - T.0, 5_000);
        assert_eq!(one_upvote - T.0, 1_800);
    }

    #[test]
    fn comment_bonus_collapses_once_the_room_turns_against_it() {
        // Just above the cutoff, comments are worth full value...
        let above = postmill_ranking(-4, 10, T);
        // ...at the cutoff they are worth a tenth.
        let at_cutoff = postmill_ranking(-5, 10, T);
        assert_eq!(above - T.0, -4 * 1_800 + 10 * 5_000);
        assert_eq!(at_cutoff - T.0, -5 * 1_800 + 10 * 500);
        assert!(
            at_cutoff < above,
            "a flamewar under a disliked post must not outrank a liked one"
        );
    }

    #[test]
    fn the_downvote_valve_dampens_a_flamewar_but_does_not_stop_one() {
        // The valve is real: the SAME thread scores far lower once disliked.
        let liked = postmill_ranking(1, 200, T);
        let hated = postmill_ranking(-20, 200, T);
        assert!(hated < liked);

        // But it is a tax, not a wall — 200 replies still beat a modest post.
        // -20 * 1800 + 200 * 500 = +64_000, vs 1 * 1800 + 3 * 5000 = +16_800.
        let modest = postmill_ranking(1, 3, T);
        assert!(
            hated > modest,
            "documents the real limit: a big enough flamewar still wins ({hated} > {modest})"
        );
    }

    #[test]
    fn how_much_a_flamewar_costs_a_disliked_post() {
        let modest = postmill_ranking(1, 3, T);
        // At -20, the break-even against that modest post is 106 comments:
        // -36_000 + 500c > 16_800  =>  c > 105.6
        assert!(postmill_ranking(-20, 105, T) < modest);
        assert!(postmill_ranking(-20, 106, T) > modest);
        // ...and 245 comments to reach the cap a liked post reaches with 18.
        assert!(postmill_ranking(-20, 245, T) - T.0 == MAX_ADVANTAGE);
        assert!(postmill_ranking(-20, 244, T) - T.0 < MAX_ADVANTAGE);
    }

    #[test]
    fn advantage_is_capped_at_one_day() {
        // 18 comments already exceeds the cap (18 * 5000 = 90_000 > 86_400).
        assert_eq!(postmill_ranking(0, 18, T) - T.0, MAX_ADVANTAGE);
        assert_eq!(postmill_ranking(10_000, 10_000, T) - T.0, MAX_ADVANTAGE);
    }

    #[test]
    fn penalty_is_capped_at_half_a_day() {
        assert_eq!(postmill_ranking(-10_000, 0, T) - T.0, -MAX_PENALTY);
    }

    #[test]
    fn cap_costs_48_upvotes_but_only_18_comments() {
        assert_eq!(postmill_ranking(48, 0, T) - T.0, MAX_ADVANTAGE);
        assert!(postmill_ranking(47, 0, T) - T.0 < MAX_ADVANTAGE);
        assert_eq!(postmill_ranking(0, 18, T) - T.0, MAX_ADVANTAGE);
        assert!(postmill_ranking(0, 17, T) - T.0 < MAX_ADVANTAGE);
    }

    #[test]
    fn a_fresh_post_outranks_a_capped_one_after_a_day() {
        let old_and_great = postmill_ranking(1_000, 1_000, Timestamp(0));
        let brand_new = postmill_ranking(0, 0, Timestamp(MAX_ADVANTAGE + 1));
        assert!(
            brand_new > old_and_great,
            "nothing may squat on the front page past the cap"
        );
    }

    #[test]
    fn saturates_instead_of_overflowing() {
        // Absurd inputs must not panic in release-mode wrapping or debug overflow.
        let _ = postmill_ranking(i64::MAX, u64::MAX, Timestamp(i64::MAX));
        let _ = postmill_ranking(i64::MIN, u64::MAX, Timestamp(i64::MIN));
    }
}
