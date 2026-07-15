pub struct RuleView {
    pub id: u64,
    pub text: String,
    /// The rule's ban term, already localized (e.g. "ban: 30 d" or
    /// "ban: community max" when the rule inherits the community ceiling).
    pub ban_term: String,
}
