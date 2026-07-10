use federation::AuthError;

/// The outcome of ingesting a batch of events from a peer.
#[derive(Debug, Default)]
pub struct Ingested {
    /// How many events were authorized and applied.
    pub applied: u64,
    /// Why each rejected event was refused (authorization never silently drops).
    pub rejected: Vec<AuthError>,
}
