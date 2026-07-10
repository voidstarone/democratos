//! Blend neighbour lists into per-candidate affinity scores.

use std::collections::{HashMap, HashSet};

use crate::PostId;

/// Aggregate the neighbours of every post a user liked into one affinity score
/// per candidate post: a candidate's score is the sum of its similarities to
/// the liked posts (a candidate similar to several liked posts ranks higher).
/// Posts in `exclude` — typically everything the user has already voted on — are
/// dropped so the feed only ever surfaces something new.
pub fn blend(
    neighbour_lists: &[Vec<(PostId, f32)>],
    exclude: &HashSet<PostId>,
) -> HashMap<PostId, f32> {
    let mut scored: HashMap<PostId, f32> = HashMap::new();
    for list in neighbour_lists {
        for &(candidate, sim) in list {
            if exclude.contains(&candidate) {
                continue;
            }
            *scored.entry(candidate).or_insert(0.0) += sim;
        }
    }
    scored
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ItemIndex, PostId, Rating, UserId};

    fn rating(user: u64, post: u64, value: f32) -> Rating {
        Rating {
            user: UserId(user),
            post: PostId(post),
            value,
        }
    }

    // Two cohorts: users 1–3 upvote posts 1 & 2 together; users 4–5 upvote
    // posts 3 & 4 together. Posts 1↔2 and 3↔4 should be each other's neighbours.
    fn two_cohorts() -> Vec<Rating> {
        let mut v = Vec::new();
        for u in 1..=3 {
            v.push(rating(u, 1, 1.0));
            v.push(rating(u, 2, 1.0));
            v.push(rating(u, 3, -1.0)); // dislike the other cohort's posts
        }
        for u in 4..=5 {
            v.push(rating(u, 3, 1.0));
            v.push(rating(u, 4, 1.0));
            v.push(rating(u, 1, -1.0)); // variance: dislike the other cohort's post
        }
        v
    }

    #[test]
    fn blend_recommends_co_liked_and_excludes_seen() {
        let index = ItemIndex::build(&two_cohorts(), 10);
        // A new user who liked post 1 — expect post 2 recommended, post 1 not.
        let lists = vec![index.neighbours(PostId(1)).to_vec()];
        let exclude: HashSet<PostId> = [PostId(1)].into_iter().collect();
        let scored = blend(&lists, &exclude);
        assert!(
            scored.contains_key(&PostId(2)),
            "recommends co-liked post 2"
        );
        assert!(
            !scored.contains_key(&PostId(1)),
            "excludes already-liked post 1"
        );
    }
}
