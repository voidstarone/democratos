/// The signed-in user's standing within this demos.
pub struct StandingView {
    pub joined: bool,
    pub tier: String,
    pub is_voter: bool,
    pub eligible: bool,
    pub contribution: i64,
    /// Human-readable, translated unmet franchise requirements.
    pub unmet: Vec<String>,
    /// Set when eligible but throttled by the Layer-2 rate cap.
    pub queued_note: Option<String>,
}
