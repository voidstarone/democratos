//! The federation bridge: turn this node's outbox into a **signed** change feed,
//! and apply a peer's feed only after it is **authenticated and authorized**.
//!
//! This is where the pure `federation` crypto/authorization meets the concrete
//! `PostgresStore`. It is deliberately transport-agnostic — an HTTP server hands
//! [`changes_since`] output to peers, and an HTTP client feeds what it pulls into
//! [`Replicator::ingest`] — so the security-critical logic has one home and is
//! testable without any network.
//!
//! [`Replicator::ingest`] is the resolution of review #1: every event passes
//! [`federation::authorize`] (signature + rightful-owner + non-stale-epoch)
//! *before* it can reach the store's apply path. An event that is unsigned,
//! forged, from a non-owner, or from a fenced old owner is dropped, never applied.

pub mod changes_since;
pub mod command;
pub mod http;
pub mod ingested;
pub mod replicator;

pub use command::command::Command;
pub use command::command_outcome::CommandOutcome;
pub use command::command_transport::CommandTransport;
pub use command::auth_rate_limiter::AuthRateLimiter;
pub use command::in_memory_rate_limit_store::InMemoryRateLimitStore;
pub use command::rate_limit_store::RateLimitStore;
pub use command::execute::execute;
pub use command::federated_authenticator::FederatedAuthenticator;
pub use command::federated_minter::FederatedMinter;
pub use command::federated_writes::FederatedWrites;
pub use command::forward_error::ForwardError;
pub use command::mint_rate_limiter::MintRateLimiter;
pub use command::http_command_transport::HttpCommandTransport;
pub use command::sync_vote_executor::SyncVoteExecutor;
pub use command::write_router::WriteRouter;

pub use http::command_client::CommandClient;
pub use http::command_router::command_router;
pub use http::command_state::CommandState;
pub use http::feed_client::FeedClient;
pub use http::feed_router::feed_router;
pub use http::feed_state::FeedState;
pub use http::ingest_client::IngestClient;
pub use http::ingest_router::ingest_router;
pub use http::ingest_state::IngestState;
pub use http::peer::Peer;
pub use http::poll_peer::poll_peer;
pub use http::serve_federation::serve_federation;
pub use http::serve_feed::serve_feed;
pub use http::spawn_puller::spawn_puller;

pub use changes_since::changes_since;
pub use ingested::Ingested;
pub use replicator::Replicator;
