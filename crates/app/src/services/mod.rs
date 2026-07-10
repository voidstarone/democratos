//! Use-cases — the application's public API. Both the web and CLI adapters call
//! these exact methods; neither contains business logic of its own.
//!
//! One definition per file: the [`Services`](services::Services) container and
//! each supporting type live in their own leaf module. The crate root re-exports
//! the flat names.

pub mod enfranchise_outcome;
pub mod feed_item;
pub mod member_metrics;
pub mod search_results;
pub mod search_scope;
#[allow(clippy::module_inception)]
pub mod services;
