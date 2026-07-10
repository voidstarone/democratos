pub struct ReportRow {
    pub id: u64,
    pub summary: String,
    /// One label per distinct reason the target has been flagged for.
    pub reasons: Vec<String>,
    pub reporter: String,
    /// `Some(trial_id)` once a trial is under way.
    pub on_trial: Option<u64>,
}
