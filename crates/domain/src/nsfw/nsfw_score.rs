//! The NSFW text suspicion score.

/// Strong indicators — a single occurrence flags on its own.
const STRONG: &[&str] = &["porn", "pornographic", "xxx", "hardcore", "nsfw"];
/// Moderate indicators — two together cross the threshold.
const MODERATE: &[&str] = &[
    "explicit", "nude", "nudes", "nudity", "sex", "sexual", "erotic", "erotica",
];
/// Weak indicators — context words that only add up.
const WEAK: &[&str] = &["lewd", "fetish", "suggestive"];

const STRONG_WEIGHT: u32 = 50;
const MODERATE_WEIGHT: u32 = 25;
const WEAK_WEIGHT: u32 = 15;

/// NSFW suspicion score for a blob of text, 0 (clearly safe) to 100. Matching is
/// whole-token and case-insensitive (so "essex" does not match "sex"), and each
/// distinct lexicon term counts once, so a single word repeated cannot inflate
/// the score.
pub fn nsfw_score(text: &str) -> u8 {
    let tokens: std::collections::HashSet<String> = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_ascii_lowercase())
        .collect();

    let mut score: u32 = 0;
    for tok in &tokens {
        if STRONG.contains(&tok.as_str()) {
            score += STRONG_WEIGHT;
        } else if MODERATE.contains(&tok.as_str()) {
            score += MODERATE_WEIGHT;
        } else if WEAK.contains(&tok.as_str()) {
            score += WEAK_WEIGHT;
        }
    }
    score.min(100) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{is_nsfw_text, NSFW_FLAG_THRESHOLD};

    #[test]
    fn safe_text_scores_zero() {
        assert_eq!(nsfw_score("a friendly post about gardening in essex"), 0);
        assert!(!is_nsfw_text("the sussex coast is lovely"));
    }

    #[test]
    fn strong_term_flags_on_its_own() {
        assert!(nsfw_score("free porn here") >= NSFW_FLAG_THRESHOLD);
        assert!(is_nsfw_text("tagged nsfw"));
    }

    #[test]
    fn two_moderate_terms_cross_threshold() {
        assert!(nsfw_score("explicit nude content") >= NSFW_FLAG_THRESHOLD); // 25 + 25
        assert!(nsfw_score("nude photo").lt(&NSFW_FLAG_THRESHOLD)); // single moderate = 25
    }

    #[test]
    fn repeated_word_does_not_inflate() {
        // "porn" many times still counts once (distinct tokens).
        assert_eq!(nsfw_score("porn porn porn porn"), STRONG_WEIGHT as u8);
    }

    #[test]
    fn score_is_capped() {
        assert_eq!(nsfw_score("porn xxx hardcore explicit nude sexual"), 100);
    }
}
