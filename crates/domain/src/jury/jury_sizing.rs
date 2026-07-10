//! How a demos sizes the jury that judges a report.

use serde::{Deserialize, Serialize};

use crate::ContentScale;

/// How a demos sizes the jury that judges a report — a *governable* policy
/// ([`crate::ProposalKind::SetJurySizing`]).
///
/// Whatever the law, the result is clamped to a strict **minority** of the
/// electorate (never half or more), and a demos too small to seat a minority
/// panel (fewer than 3 voters) holds no jury. The platform default is
/// [`JurySizing::Sqrt`]: the panel is a tiny share of a large demos and a larger
/// share of a small one, so big communities aren't dragged into every report.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum JurySizing {
    /// Sub-linear: `target = factor × ⌊√voters⌋`, the factor in basis points
    /// (`10_000` = ×1.0). A 10k-voter demos seats ~1%; a 100-voter demos ~10%.
    Sqrt {
        post_factor_bp: u32,
        comment_factor_bp: u32,
    },
    /// Linear: `target = proportion × voters` — the same share at every size.
    Proportion { post_bp: u32, comment_bp: u32 },
    /// A fixed panel size, independent of community size.
    Fixed { post: u32, comment: u32 },
}

impl Default for JurySizing {
    fn default() -> Self {
        // ×1.0·√n for posts, ×0.5·√n for comments.
        JurySizing::Sqrt {
            post_factor_bp: 10_000,
            comment_factor_bp: 5_000,
        }
    }
}

impl JurySizing {
    /// The number of jurors to empanel for `scale` content given `voters`
    /// enfranchised citizens, clamped to a strict minority of the electorate.
    /// Returns `0` when the demos is too small to seat a minority panel.
    pub fn jury_size(&self, voters: u64, scale: ContentScale) -> usize {
        // The largest panel strictly smaller than half the electorate. With
        // fewer than 3 voters this is 0 — no jury can be both ≥1 and a minority.
        let cap = voters.saturating_sub(1) / 2;
        if cap == 0 {
            return 0;
        }
        let target = match (*self, scale) {
            (JurySizing::Sqrt { post_factor_bp, .. }, ContentScale::Post) => {
                voters.isqrt() * post_factor_bp as u64 / 10_000
            }
            (
                JurySizing::Sqrt {
                    comment_factor_bp, ..
                },
                ContentScale::Comment,
            ) => voters.isqrt() * comment_factor_bp as u64 / 10_000,
            (JurySizing::Proportion { post_bp, .. }, ContentScale::Post) => {
                voters * post_bp as u64 / 10_000
            }
            (JurySizing::Proportion { comment_bp, .. }, ContentScale::Comment) => {
                voters * comment_bp as u64 / 10_000
            }
            (JurySizing::Fixed { post, .. }, ContentScale::Post) => post as u64,
            (JurySizing::Fixed { comment, .. }, ContentScale::Comment) => comment as u64,
        };
        target.clamp(1, cap) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_panel_shrinks_as_a_share_as_the_demos_grows() {
        let p = JurySizing::default();
        // Big demos: a tiny minority (~1% of 10k, ~10% of 100).
        assert_eq!(p.jury_size(10_000, ContentScale::Post), 100);
        assert_eq!(p.jury_size(100, ContentScale::Post), 10);
        // Small demos: a larger share, but still a strict minority.
        assert_eq!(p.jury_size(10, ContentScale::Post), 3);
        assert!(p.jury_size(10, ContentScale::Post) < 5);
    }

    #[test]
    fn comments_draw_a_smaller_panel_than_posts() {
        let p = JurySizing::default();
        assert_eq!(p.jury_size(100, ContentScale::Post), 10);
        assert_eq!(p.jury_size(100, ContentScale::Comment), 5);
    }

    #[test]
    fn the_panel_is_never_a_majority() {
        let p = JurySizing::default();
        for voters in 0..200u64 {
            let n = p.jury_size(voters, ContentScale::Post) as u64;
            assert!(
                2 * n < voters || n == 0,
                "{n} of {voters} is not a minority"
            );
        }
    }

    #[test]
    fn tiny_demoi_seat_no_jury() {
        let p = JurySizing::default();
        assert_eq!(p.jury_size(0, ContentScale::Post), 0);
        assert_eq!(p.jury_size(2, ContentScale::Post), 0); // no minority ≥ 1 exists
        assert_eq!(p.jury_size(3, ContentScale::Post), 1);
    }

    #[test]
    fn proportion_and_fixed_laws_also_stay_a_minority() {
        // 60% requested, but clamped below half.
        let prop = JurySizing::Proportion {
            post_bp: 6_000,
            comment_bp: 6_000,
        };
        assert_eq!(prop.jury_size(10, ContentScale::Post), 4); // not 6

        // A fixed 50 on a 20-voter demos is clamped to the largest minority (9).
        let fixed = JurySizing::Fixed {
            post: 50,
            comment: 50,
        };
        assert_eq!(fixed.jury_size(20, ContentScale::Post), 9);
        // ...and honoured outright where it is already a minority.
        assert_eq!(fixed.jury_size(1_000, ContentScale::Post), 50);
    }
}
