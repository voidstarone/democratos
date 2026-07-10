//! A parent entity whose community a ballot inherits.

/// A parent entity whose community a ballot inherits. Ballot rows (`votes`,
/// `post_votes`, `jury_ballots`) do not carry a `demos_id` of their own — their
/// community is that of the proposal / post / trial they attach to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ParentKind {
    Proposal,
    Post,
    Trial,
}
