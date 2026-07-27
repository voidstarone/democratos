//! The media-ingest safety layer: the allowlist that decides what an upload may
//! be, the magic-number sniff that decides what it *actually* is, the limits that
//! bound it, and the guard that runs every upload through sanitize → scan → store.
//!
//! Everything here is policy and glue, deliberately free of codec dependencies:
//! the heavy decoding work (re-encoding images, hashing for the known-bad corpus,
//! validating containers) lives in `adapter-media-safety` behind the
//! [`MediaSanitizer`](crate::MediaSanitizer) and
//! [`MediaSafetyScanner`](crate::MediaSafetyScanner) ports. That split is what
//! lets the application layer state the *rules* — which types exist, what the
//! caps are, what happens when the scanner can't answer — while a node too small
//! to decode media swaps in a cheaper adapter without touching any of it.
//!
//! The type vocabulary is closed and small: png, jpeg, gif, webp, mp4, webm.
//! Notably **no SVG** — an SVG is a script-bearing document, and serving one from
//! our own origin would hand an uploader script execution there.
//!
//! See `docs/media-safety.md` for the operator-facing description of this
//! pipeline and the CSAM-scanning posture.

pub mod content_type_for;
pub mod extension_for;
pub mod guarded_media_store;
pub mod is_allowed;
pub mod max_image_pixels;
pub mod max_upload_bytes;
pub mod media_key;
pub mod scan_failure_policy;
pub mod sniff_content_type;
pub mod upload_matches_bytes;
