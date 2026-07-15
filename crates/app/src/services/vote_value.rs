/// The signed integer value of an up/down/clear vote: `+1` up, `-1` down, `0`
/// none. Used to subtract a member's own ballot from their contribution metric.
pub(super) fn vote_value(dir: Option<bool>) -> i64 {
    match dir {
        Some(true) => 1,
        Some(false) => -1,
        None => 0,
    }
}
