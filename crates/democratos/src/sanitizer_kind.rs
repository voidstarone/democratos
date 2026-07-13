//! Which media sanitizer a node runs.

use clap::ValueEnum;

/// Which [`MediaSanitizer`](app::MediaSanitizer) the ingest pipeline uses.
#[derive(Clone, Copy, ValueEnum)]
pub(crate) enum SanitizerKind {
    /// Decode and re-encode images (strips metadata, defuses bombs/polyglots);
    /// structurally validate video. The default and the safe choice.
    Reencode,
    /// Validate the type without decoding/re-encoding. Lighter on CPU for a very
    /// small box, but stored bytes remain attacker-influenced.
    Passthrough,
}
