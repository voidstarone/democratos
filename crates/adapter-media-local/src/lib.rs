//! Local-filesystem implementation of the [`MediaStore`] port.
//!
//! Uploaded bytes are written under a directory using a content-addressed key
//! (`<hash>.<ext>`) computed by [`app::media_key`], and served back by that key
//! through the web layer's `/media/:key` route. Because the post only ever
//! stores the **URL** `put` returns, replacing this with an S3/CDN adapter is a
//! single `Arc::new(...)` change in the composition root.
//!
//! One definition per file: each public type lives in its own leaf module and is
//! re-exported flat here.

pub mod disabled_media_store;
pub mod local_media_store;

pub use disabled_media_store::DisabledMediaStore;
pub use local_media_store::LocalMediaStore;
