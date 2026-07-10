/// Looser allowance for every other state-changing POST (found, post, vote,
/// report, …). High enough that normal interactive use never trips it, low
/// enough to cap scripted floods.
pub const WRITE_MAX_REQUESTS: u32 = 120;
