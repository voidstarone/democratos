use domain::ProposalKind;

pub(super) fn voting_window_days(kind: &ProposalKind) -> i64 {
    use domain::DecisionClass::*;
    match kind.decision_class() {
        Moderation => 3,
        RuleChange => 5,
        BanOrRecall => 5,
        Constitutional => 7,
    }
}
