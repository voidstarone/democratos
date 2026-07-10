//! A precomputed item→item similarity model.

use std::collections::HashMap;

use crate::{PostId, Rating, UserId};

/// A precomputed item→item similarity model: for each post, its most similar
/// posts in descending order. Built once from a ratings snapshot and then
/// served by O(1) lookups, so the request path never recomputes similarity.
#[derive(Clone, Debug, Default)]
pub struct ItemIndex {
    neighbours: HashMap<PostId, Vec<(PostId, f32)>>,
}

impl ItemIndex {
    /// Build the model from a full snapshot of ratings, keeping the top `k`
    /// neighbours per post.
    ///
    /// Uses **adjusted cosine** similarity: each user's ratings are mean-centred
    /// first, which removes per-user bias (a user who upvotes everything carries
    /// no information). Similarity is computed only for post pairs that share a
    /// voter — discovered through an inverted index — so the cost scales with
    /// the *sparse* co-rating structure, never with `posts²`. Only positively
    /// correlated neighbours are kept.
    pub fn build(ratings: &[Rating], k: usize) -> Self {
        // 1. Each user's mean rating, to centre by.
        let mut totals: HashMap<UserId, (f32, u32)> = HashMap::new();
        for r in ratings {
            let e = totals.entry(r.user).or_insert((0.0, 0));
            e.0 += r.value;
            e.1 += 1;
        }

        // 2. Centred ratings grouped by user — the inverted index that lets us
        //    enumerate only co-rated pairs.
        let mut by_user: HashMap<UserId, Vec<(PostId, f32)>> = HashMap::new();
        for r in ratings {
            let (sum, count) = totals[&r.user];
            let centred = r.value - sum / count as f32;
            by_user.entry(r.user).or_default().push((r.post, centred));
        }

        // 3. Accumulate squared norms per post and dot products per co-rated
        //    pair (keyed low→high so each unordered pair is counted once).
        let mut norm_sq: HashMap<PostId, f32> = HashMap::new();
        let mut dot: HashMap<(PostId, PostId), f32> = HashMap::new();
        for items in by_user.values() {
            for &(post, c) in items {
                *norm_sq.entry(post).or_insert(0.0) += c * c;
            }
            for i in 0..items.len() {
                for j in (i + 1)..items.len() {
                    let (a, ca) = items[i];
                    let (b, cb) = items[j];
                    let key = if a.0 <= b.0 { (a, b) } else { (b, a) };
                    *dot.entry(key).or_insert(0.0) += ca * cb;
                }
            }
        }

        // 4. Cosine for each pair; record both directions.
        let mut neighbours: HashMap<PostId, Vec<(PostId, f32)>> = HashMap::new();
        for ((a, b), d) in dot {
            let denom = norm_sq[&a].sqrt() * norm_sq[&b].sqrt();
            if denom == 0.0 {
                continue;
            }
            let sim = d / denom;
            if sim <= 0.0 {
                continue; // keep only positive correlations
            }
            neighbours.entry(a).or_default().push((b, sim));
            neighbours.entry(b).or_default().push((a, sim));
        }

        // 5. Keep the strongest `k` neighbours per post (id breaks ties for a
        //    deterministic, auditable model).
        for list in neighbours.values_mut() {
            list.sort_by(|x, y| {
                y.1.partial_cmp(&x.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(x.0 .0.cmp(&y.0 .0))
            });
            list.truncate(k);
        }

        Self { neighbours }
    }

    /// The precomputed neighbours of `post`, strongest first.
    pub fn neighbours(&self, post: PostId) -> &[(PostId, f32)] {
        self.neighbours.get(&post).map_or(&[], Vec::as_slice)
    }

    /// Iterate every post with its neighbour list. Lets a persistence adapter
    /// serialise the model without owning its internal layout.
    pub fn entries(&self) -> impl Iterator<Item = (PostId, &[(PostId, f32)])> {
        self.neighbours
            .iter()
            .map(|(post, list)| (*post, list.as_slice()))
    }

    /// Number of posts that have at least one neighbour. Lets callers detect an
    /// empty (not-yet-useful) model.
    pub fn len(&self) -> usize {
        self.neighbours.len()
    }

    pub fn is_empty(&self) -> bool {
        self.neighbours.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn co_liked_posts_are_neighbours() {
        let index = ItemIndex::build(&two_cohorts(), 10);
        let n1: Vec<PostId> = index.neighbours(PostId(1)).iter().map(|n| n.0).collect();
        assert!(n1.contains(&PostId(2)), "post 1 should neighbour post 2");
        let n3: Vec<PostId> = index.neighbours(PostId(3)).iter().map(|n| n.0).collect();
        assert!(n3.contains(&PostId(4)), "post 3 should neighbour post 4");
    }

    #[test]
    fn top_k_truncates_and_sorts() {
        let index = ItemIndex::build(&two_cohorts(), 1);
        for post in 1..=4 {
            assert!(
                index.neighbours(PostId(post)).len() <= 1,
                "k=1 caps each list at one neighbour"
            );
        }
    }

    #[test]
    fn similarity_is_positive_and_bounded() {
        let index = ItemIndex::build(&two_cohorts(), 10);
        for &(_, sim) in index.neighbours(PostId(1)) {
            assert!(sim > 0.0 && sim <= 1.0 + 1e-6, "cosine in (0, 1]: {sim}");
        }
    }

    #[test]
    fn empty_index_for_no_ratings() {
        assert!(ItemIndex::build(&[], 10).is_empty());
    }
}
