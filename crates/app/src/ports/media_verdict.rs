//! How the media scanner classifies a piece of uploaded media.

/// How the media scanner classifies a piece of uploaded media.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MediaVerdict {
    Sfw,
    Nsfw,
    /// The scanner can't decide (e.g. a stub with no model) — the caller falls
    /// back to text/tag signals rather than flagging.
    Unknown,
}
