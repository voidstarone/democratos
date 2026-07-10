use app::StoreError;

/// Why a forwarded write could not be completed.
#[derive(Debug)]
pub enum ForwardError {
    /// The target community has no current owner — nobody can authorize the write.
    Unowned,
    /// The owner could not be reached (fail-closed; the write was NOT applied).
    OwnerUnreachable(String),
    /// The owner ran the use-case and rejected it (a domain error, e.g. AlreadyVoted).
    Rejected(String),
    /// An infrastructure/store failure resolving or executing the command locally.
    /// Carries the typed [`StoreError`] so a store outcome (e.g. `AlreadyVoted`)
    /// survives the gateway rather than being flattened to a string.
    App(StoreError),
}

impl std::fmt::Display for ForwardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ForwardError::Unowned => write!(f, "community has no reachable owner"),
            ForwardError::OwnerUnreachable(e) => write!(f, "owner unreachable: {e}"),
            ForwardError::Rejected(e) => write!(f, "owner rejected the write: {e}"),
            ForwardError::App(e) => write!(f, "{e}"),
        }
    }
}
