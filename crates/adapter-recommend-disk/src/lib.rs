//! Disk-backed similarity index — a [`SimilarityIndex`] whose *steady-state*
//! memory is flat regardless of catalogue size.
//!
//! Where [`adapter-recommend-memory`](../adapter_recommend_memory/index.html)
//! keeps the whole post→post map resident, this adapter writes each post's
//! neighbour list to a flat binary file and keeps only a small **offset table**
//! (`PostId → (offset, len)`, ~16 bytes per post) in RAM. A neighbour lookup
//! reads one short blob from the file by offset, so serving 1M posts costs a
//! ~16 MB table instead of a ~1 GB map.
//!
//! Concurrency: the offset table, file handle, and version live behind an
//! `RwLock`. Reads take the read lock only long enough to clone the `Arc<File>`,
//! then read by absolute offset (`pread`, no shared cursor), so lookups never
//! block one another. A rebuild writes a fresh temp file, fsyncs, atomically
//! renames it over the old one, and swaps the state under a brief write lock.
//!
//! Caveat — this bounds *serving* memory, not *rebuild* peak: building the
//! model still materialises [`domain::ItemIndex`] in RAM transiently before it
//! is streamed to disk.
//!
//! On-disk format: entries concatenated back to back; each is `u32` little-endian
//! neighbour count followed by that many `(u64 post id, f32 similarity)` pairs,
//! all little-endian. The offset table is rebuilt in memory from the same data,
//! so the file alone is sufficient to serve.
//!
//! One definition per file: each public item lives in its own leaf module and is
//! re-exported flat here.

pub mod default_neighbours;
pub mod disk_recommender;

pub use default_neighbours::DEFAULT_NEIGHBOURS;
pub use disk_recommender::DiskRecommender;
