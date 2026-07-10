//! The curated corpus of known-bad hashes the scanner matches against.

use std::collections::HashSet;
use std::path::Path;

use crate::perceptual_hash::hamming_distance;

/// The maximum Hamming distance at which a perceptual (dHash) fingerprint counts
/// as a match. 0 is byte-identical after re-encoding; a small threshold catches
/// resized/recompressed copies while keeping false positives negligible. Kept
/// deliberately tight — this corpus describes *illegal* content, so a loose
/// threshold that flags innocent images is its own harm.
const DEFAULT_PERCEPTUAL_THRESHOLD: u32 = 8;

/// A curated set of known-bad content fingerprints, loaded from an operator-
/// supplied file. Two kinds of entry are supported, one per line:
///
/// * `sha256:<64 hex>` — an exact, cryptographic match (a byte-identical copy).
/// * `dhash:<16 hex>` — a perceptual match (a visually similar copy), matched
///   within [`DEFAULT_PERCEPTUAL_THRESHOLD`] bits.
///
/// Bare `#` lines and blanks are ignored. This is the seam for a real source: an
/// operator with lawful access to NCMEC / PhotoDNA hash sets converts them into
/// this format. The file itself contains only opaque hashes — never any imagery —
/// so it is safe to ship and store.
#[derive(Debug, Default, Clone)]
pub struct KnownHashSet {
    sha256: HashSet<[u8; 32]>,
    dhash: Vec<u64>,
    threshold: u32,
}

impl KnownHashSet {
    /// An empty corpus — matches nothing. Used when no hash file is configured.
    pub fn empty() -> Self {
        Self {
            sha256: HashSet::new(),
            dhash: Vec::new(),
            threshold: DEFAULT_PERCEPTUAL_THRESHOLD,
        }
    }

    /// Load a corpus from a file. Unparseable lines are skipped with a warning so
    /// one malformed row can't blind the whole list.
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(Self::parse(&text))
    }

    /// Parse a corpus from the file's text (also the unit-test seam).
    pub fn parse(text: &str) -> Self {
        let mut set = Self::empty();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (kind, value) = line.split_once(':').unwrap_or(("", line));
            match kind {
                "sha256" => match decode_hex_32(value) {
                    Some(h) => {
                        set.sha256.insert(h);
                    }
                    None => tracing::warn!(line = n + 1, "skipping malformed sha256 hash entry"),
                },
                "dhash" => match u64::from_str_radix(value.trim(), 16) {
                    Ok(h) => set.dhash.push(h),
                    Err(_) => tracing::warn!(line = n + 1, "skipping malformed dhash entry"),
                },
                _ => tracing::warn!(line = n + 1, "skipping unrecognised hash entry"),
            }
        }
        set
    }

    /// Whether the corpus holds no entries — the scanner is effectively a no-op.
    pub fn is_empty(&self) -> bool {
        self.sha256.is_empty() && self.dhash.is_empty()
    }

    pub fn len(&self) -> usize {
        self.sha256.len() + self.dhash.len()
    }

    /// Whether these exact bytes match a known cryptographic hash.
    pub fn contains_sha256(&self, digest: &[u8; 32]) -> bool {
        self.sha256.contains(digest)
    }

    /// Whether a perceptual fingerprint is within the match threshold of any known
    /// dHash.
    pub fn matches_perceptual(&self, fingerprint: u64) -> bool {
        self.dhash
            .iter()
            .any(|&known| hamming_distance(known, fingerprint) <= self.threshold)
    }
}

/// Decode exactly 32 bytes of hex, or `None`.
fn decode_hex_32(s: &str) -> Option<[u8; 32]> {
    let s = s.trim();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_entry_kinds_and_ignores_noise() {
        let text = "\
            # a comment\n\
            \n\
            sha256:0000000000000000000000000000000000000000000000000000000000000001  # trailing\n\
            dhash:00000000000000ff\n\
            garbage line\n\
            sha256:tooshort\n";
        let set = KnownHashSet::parse(text);
        assert_eq!(set.len(), 2);
        let mut exact = [0u8; 32];
        exact[31] = 1;
        assert!(set.contains_sha256(&exact));
        assert!(set.matches_perceptual(0x00000000000000ff));
        assert!(!set.matches_perceptual(0xffffffffffffffff));
    }

    #[test]
    fn empty_corpus_matches_nothing() {
        let set = KnownHashSet::empty();
        assert!(set.is_empty());
        assert!(!set.contains_sha256(&[0u8; 32]));
        assert!(!set.matches_perceptual(0));
    }
}
