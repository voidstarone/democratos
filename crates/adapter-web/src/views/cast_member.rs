//! One seat in the trial-theater cast (dev walk-through).

/// One participant in the seeded trial, with the seat they occupy and a link to
/// assume their session. Purely a dev-tool row (see the trial theater).
pub struct CastMember {
    pub id: u64,
    pub handle: String,
    /// Human role label: "Accused", "Reporter", "Juror", or "Voter (bystander)".
    pub role: String,
    /// Whether this seat is the browser's current session.
    pub is_current: bool,
    /// Whether this seat sits on the jury (drives whether a verdict is expected).
    pub is_juror: bool,
    /// For a juror: whether they have already returned their verdict.
    pub has_voted: bool,
}
