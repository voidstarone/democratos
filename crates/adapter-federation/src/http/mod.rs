//! HTTP transport for the change feed: a node **serves** its signed feed and
//! **pulls** its peers' feeds. A thin layer over [`changes_since`](crate::changes_since)
//! and [`Replicator::ingest`](crate::Replicator::ingest) — all the security lives
//! there; this module only moves bytes.
//!
//! # Node-to-node auth
//!
//! Events are already signed, so their authenticity does not depend on the
//! transport. Still, the feed and (later) forwarding endpoints are node-only:
//! when a cluster shares a bearer token, requests must carry it, and the feed
//! server binds a **separate address** from the public web UI so it can be
//! firewalled to the node network. Deploy it behind TLS.

pub mod command_client;
pub mod command_router;
pub mod command_state;
pub mod feed_client;
pub mod feed_error;
pub mod feed_router;
pub mod feed_state;
pub mod ingest_client;
pub mod ingest_router;
pub mod ingest_state;
pub mod peer;
pub mod poll_peer;
pub mod serve_federation;
pub mod serve_feed;
pub mod spawn_puller;

pub(crate) mod bearer_ok;
