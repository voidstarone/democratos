//! What a [`MediaSafetyScanner`](crate::MediaSafetyScanner) concludes about media.

/// The outcome of scanning media bytes for known illegal content (CSAM).
///
/// This is deliberately coarse: the scanner reports a *match against a known-bad
/// corpus*, not a probability. A `Match` means the bytes matched an entry the
/// operator has curated (a cryptographic or perceptual hash from a source such as
/// NCMEC / PhotoDNA), and the pipeline must block and preserve. Anything the
/// scanner cannot positively match is [`Clear`](SafetyVerdict::Clear) — a scanner
/// that is *unavailable* returns an error instead, so the ingest policy can decide
/// whether to fail closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyVerdict {
    /// No entry in the known-bad corpus matched these bytes.
    Clear,
    /// The bytes matched a known-bad entry and must be blocked and preserved.
    Match {
        /// Which corpus / list the hit came from, for the incident record.
        source: String,
        /// A short machine reason (e.g. `"sha256"`, `"perceptual"`) — never any
        /// detail that could describe the matched material.
        reason: String,
    },
}

impl SafetyVerdict {
    /// A convenience constructor for a positive match.
    pub fn matched(source: impl Into<String>, reason: impl Into<String>) -> Self {
        SafetyVerdict::Match {
            source: source.into(),
            reason: reason.into(),
        }
    }

    /// Whether this verdict blocks the upload.
    pub fn is_match(&self) -> bool {
        matches!(self, SafetyVerdict::Match { .. })
    }
}
