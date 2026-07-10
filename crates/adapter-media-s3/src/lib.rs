//! S3-compatible (AWS S3 / MinIO / Ceph) implementation of the [`MediaStore`]
//! port — the object store that lets **any node serve any upload**.
//!
//! # Why this exists
//!
//! [`adapter-media-local`](../adapter_media_local/index.html) writes bytes to one
//! node's local disk, so an upload accepted by node A is invisible to node B. In
//! a federation that replicates a community onto several nodes, a post's image
//! must be fetchable wherever the post is read. This adapter puts every upload in
//! **one shared bucket** that all nodes read and write, so media is as global as
//! the replicated rows that reference it.
//!
//! # URL modes
//!
//! * **Proxied (default)** — `put` stores the object and returns the same
//!   `/media/<key>` URL the local store returns; the web layer's `/media/:key`
//!   route calls [`get`](MediaStore::get), which fetches the bytes from the
//!   bucket. Identical URLs to the local store, and the bucket stays private
//!   behind the app.
//! * **Direct/CDN** — when `public_base` is set, `put` returns
//!   `<public_base>/<key>` and [`get`](MediaStore::get) returns `Ok(None)`,
//!   because a CDN (or the bucket itself) serves the bytes. The web layer then
//!   never proxies media.
//!
//! # Content addressing
//!
//! Keys come from [`app::media_key`] (`"<hash>.<ext>"`), so identical uploads
//! dedupe to one object and `put` skips the upload when the object already
//! exists — the same property the local store has, at bucket scope.
//!
//! One definition per file: each public type lives in its own leaf module and is
//! re-exported flat here.

pub mod s3_config;
pub mod s3_media_store;

pub use s3_config::S3Config;
pub use s3_media_store::S3MediaStore;
