//! Burn the same Argon2 verification time a real login would on the miss path,
//! so account existence can't be probed by response timing.

use std::sync::OnceLock;

use crate::auth::hash_password::hash_password;
use crate::auth::verify_password::verify_password;

/// Spend the same Argon2 verification time a real login would, then discard the
/// (always-failing) result. Call this on the *account-not-found* and
/// *credential-less-account* branches of a login so those branches take as long
/// as verifying a genuine hash. Without it, a missing email returns near-
/// instantly while a real one pays the memory-hard cost, and that timing gap
/// lets an attacker enumerate which emails have accounts. The dummy hash is
/// computed once and cached; it uses the same default Argon2 parameters as
/// [`hash_password`], so its verification cost matches a real one's.
pub fn spend_verify_time(password: &str) {
    static DUMMY_HASH: OnceLock<String> = OnceLock::new();
    let hash = DUMMY_HASH.get_or_init(|| {
        // A fixed, non-secret password: only the hashing *work* matters here,
        // never the value. `unwrap_or_default` keeps this infallible — an empty
        // hash simply makes the subsequent verify a fast reject, which at worst
        // reverts to the prior (already-shipped) behaviour on this cold path.
        hash_password("account-enumeration-timing-equalizer").unwrap_or_default()
    });
    let _ = verify_password(password, hash);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spend_verify_time_is_a_slow_noop() {
        // It must not panic and must return nothing observable — its only job is
        // to burn the same time a real verification would on the miss path.
        spend_verify_time("whatever the attacker typed");
        spend_verify_time("");
    }
}
