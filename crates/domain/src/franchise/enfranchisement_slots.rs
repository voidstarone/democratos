//! Layer 2 — the enfranchisement rate cap.

/// Layer 2 — how many *new* voters a demos may admit right now.
///
/// The voter roll may grow by at most 10% per 30 days, with a floor of 5 so tiny
/// demos can still grow. Members who qualify beyond the cap queue by
/// qualification date; nobody is ever denied, only delayed.
///
/// Returns the number of open admission slots given the current voter count and
/// how many were admitted in the trailing 30-day window.
pub fn enfranchisement_slots(voter_count: u64, admitted_last_30d: u64) -> u64 {
    const FLOOR: u64 = 5;
    // ceil(10% of voter_count), i.e. (voter_count * 10 + 99) / 100.
    let ten_percent = voter_count.saturating_mul(10).saturating_add(99) / 100;
    let cap = ten_percent.max(FLOOR);
    cap.saturating_sub(admitted_last_30d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_cap_floor_applies_to_small_demos() {
        // 3 voters: 10% rounds to 1, but the floor of 5 governs.
        assert_eq!(enfranchisement_slots(3, 0), 5);
        assert_eq!(enfranchisement_slots(3, 2), 3);
    }

    #[test]
    fn rate_cap_ten_percent_governs_large_demos() {
        // 100 voters -> cap 10; 200 -> cap 20.
        assert_eq!(enfranchisement_slots(100, 0), 10);
        assert_eq!(enfranchisement_slots(200, 5), 15);
    }

    #[test]
    fn flood_cannot_outpace_the_cap() {
        // 100 established voters, 10 already admitted this window: only 0 more,
        // no matter how many newcomers qualify.
        assert_eq!(enfranchisement_slots(100, 10), 0);
        assert_eq!(enfranchisement_slots(100, 50), 0); // saturating, never panics
    }
}
