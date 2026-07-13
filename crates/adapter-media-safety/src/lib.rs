//! Local implementations of the media-safety ports — the two halves of keeping
//! uploaded media safe, both runnable on a single small box and each swappable at
//! its port for an external service.
//!
//! **Malicious media** — [`ImageReencodeSanitizer`] decodes every upload (proving
//! it is the media it claims to be, not a polyglot), bounds it against
//! decompression/pixel bombs, and re-encodes images so stored bytes carry no
//! metadata or trailing payload. [`PassthroughSanitizer`] is a lighter,
//! non-re-encoding fallback for CPU-constrained nodes.
//!
//! **Illegal content (CSAM)** — [`HashListSafetyScanner`] matches uploads against
//! a curated [`KnownHashSet`] of cryptographic and perceptual hashes (the offline,
//! honest baseline; an external classifier plugs in at the same port).
//! [`AllowAllSafetyScanner`] is the explicit opt-out. A positive match is never
//! deleted: [`DirQuarantine`] preserves it for a NCMEC report.
//!
//! One definition per file; the crate root re-exports the flat names.

pub mod allow_all_safety_scanner;
pub mod dir_quarantine;
pub mod hash_list_safety_scanner;
pub mod image_reencode_sanitizer;
pub mod known_hash_set;
pub mod passthrough_sanitizer;
pub mod perceptual_hash;
pub mod validate_video;

pub use allow_all_safety_scanner::AllowAllSafetyScanner;
pub use dir_quarantine::DirQuarantine;
pub use hash_list_safety_scanner::HashListSafetyScanner;
pub use image_reencode_sanitizer::ImageReencodeSanitizer;
pub use known_hash_set::KnownHashSet;
pub use passthrough_sanitizer::PassthroughSanitizer;
