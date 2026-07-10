//! Node identity and the composite-ID scheme that lets a federated network of
//! nodes mint globally-unique IDs without coordinating.
//!
//! Every entity ID in the system is a `u64`. In a single-box deployment those
//! are just `1, 2, 3, …`. To federate — many nodes, each with its own database,
//! each the source of truth for the communities it hosts — two nodes must never
//! mint the same ID. We solve this *without* a coordinator by **partitioning the
//! `u64`**:
//!
//! ```text
//!  63            48 47                                   0
//! ┌────────────────┬──────────────────────────────────────┐
//! │  node (16 bit) │            sequence (48 bit)          │
//! └────────────────┴──────────────────────────────────────┘
//! ```
//!
//! * The high 16 bits name the **origin node** — the node that minted the ID.
//! * The low 48 bits are that node's local, monotonic sequence.
//!
//! So every ID is globally unique *and self-describing*: [`origin_node`](origin_node::origin_node) recovers
//! who minted it. Because a community is minted by its home node, the origin of a
//! `DemosId` is the community's **bootstrap owner** — a good default for routing,
//! though current ownership is dynamic once rehoming is in play (the control
//! plane, not the ID, is authoritative for *current* ownership).
//!
//! 48 bits is ~281 trillion IDs per node; 16 bits is 65 536 nodes. The domain ID
//! types stay plain `u64` newtypes — only the *allocation* strategy changes, so
//! nothing in the governance rules is aware federation exists.

pub mod compose_id;
pub mod local_sequence;
pub mod max_sequence;
pub mod node_id;
pub mod origin_node;
pub mod sequence_bits;
pub mod sequence_mask;
