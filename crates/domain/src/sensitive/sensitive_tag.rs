//! How a reviewer classifies flagged content.

use serde::{Deserialize, Serialize};

/// How a reviewer classifies a piece of flagged content. A case gathers one tag
/// per reviewer; once enough reviewers have weighed in, the plurality tag decides
/// the outcome (see [`tally_tags`](crate::tally_tags)).
///
/// The variants are deliberately distinct because they carry *different legal and
/// product consequences* — see [`outcome_for`](crate::outcome_for). `NotSensitive`
/// is the "this was a false flag" vote.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SensitiveTag {
    /// Not actually sensitive — a false or mistaken flag; the content is restored.
    NotSensitive,
    /// Ordinary adult/pornographic content: lawful, but not-safe-for-work. Kept up
    /// behind the existing NSFW blur/age gate rather than removed.
    Porn,
    /// Graphic death, gore, or extreme violence. Removed.
    Gore,
    /// Self-harm or suicide content. Removed (and, ideally, met with support
    /// resources for the author).
    SelfHarm,
    /// Spam or scam content. Removed.
    Spam,
    /// Sensitive/removable but none of the above. Removed.
    Other,
    /// Suspected child sexual abuse material. Removed, the bytes preserved, and the
    /// operator alerted to file a report — the gravest category, so it wins ties.
    Csam,
}

impl SensitiveTag {
    /// Every variant, for tallying.
    pub const ALL: [SensitiveTag; 7] = [
        SensitiveTag::NotSensitive,
        SensitiveTag::Porn,
        SensitiveTag::Gore,
        SensitiveTag::SelfHarm,
        SensitiveTag::Spam,
        SensitiveTag::Other,
        SensitiveTag::Csam,
    ];

    /// Tie-break precedence: when two tags draw the same number of votes, the more
    /// severe one wins, biasing a split decision toward caution (removal /
    /// escalation) rather than leaving harmful content up. `NotSensitive` is lowest,
    /// so "restore" only ever wins on a clear plurality, never on a tie.
    pub fn severity(self) -> u8 {
        match self {
            SensitiveTag::Csam => 6,
            SensitiveTag::SelfHarm => 5,
            SensitiveTag::Gore => 4,
            SensitiveTag::Spam => 3,
            SensitiveTag::Other => 2,
            SensitiveTag::Porn => 1,
            SensitiveTag::NotSensitive => 0,
        }
    }

    /// A stable machine slug (used in forms and logs).
    pub fn slug(self) -> &'static str {
        match self {
            SensitiveTag::NotSensitive => "not_sensitive",
            SensitiveTag::Porn => "porn",
            SensitiveTag::Gore => "gore",
            SensitiveTag::SelfHarm => "self_harm",
            SensitiveTag::Spam => "spam",
            SensitiveTag::Other => "other",
            SensitiveTag::Csam => "csam",
        }
    }

    /// Parse a slug back to a tag.
    pub fn from_slug(s: &str) -> Option<SensitiveTag> {
        SensitiveTag::ALL.into_iter().find(|t| t.slug() == s)
    }
}
