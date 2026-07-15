//! One accusation on a trial's charge sheet — the context a juror weighs.

/// One flag from the report behind a trial, shown to jurors so they judge with
/// the full context: what was alleged, by whom, the note left, and which rule
/// (if any) was cited.
pub struct ChargeView {
    /// Localized reason label (e.g. "rule-break", "bot", "NSFW").
    pub reason: String,
    /// The reporter's handle, or the "automatic" marker for a detector flag.
    pub by: String,
    /// The free-text note left with the flag.
    pub note: String,
    /// The cited rule's text, when the flag named a specific rule.
    pub rule_text: Option<String>,
}
