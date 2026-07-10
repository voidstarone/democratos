//! A store that persists everything to one JSON text file.
//!
//! Identical port surface to the in-memory adapter; the only difference is that
//! every mutation is flushed to disk. The whole dataset is one serializable
//! `Db` value, so "the database" is literally a text file you can open and read.
//!
//! Concurrency model: a single mutex guards the in-memory copy, and each write
//! is persisted atomically (temp file + rename). Fine for the scale this is for.

mod comment_vote_rec;
mod db;
mod jury_ballot_rec;
mod post_vote_rec;
mod text_file_store;
mod vote_rec;

pub use text_file_store::TextFileStore;
