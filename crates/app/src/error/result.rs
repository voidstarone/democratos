//! The crate's `Result` alias. Its default error is [`StoreError`](crate::StoreError)
//! — the storage vocabulary shared by every `*Store` port — so `Result<T>` reads
//! as "a store operation". Use-cases that emit richer errors name them explicitly:
//! `Result<T, CastVoteError>`, `Result<i64, VotePostError>`, etc.

use crate::error::store_error::StoreError;

pub type Result<T, E = StoreError> = std::result::Result<T, E>;
