//! Layer 1 — evaluate a member against a demos's franchise criteria.

use crate::{Eligibility, FranchiseCriteria, Membership, Timestamp, Unmet, User};

/// Layer 1 — evaluate a member against a demos's franchise criteria.
///
/// Pure: depends only on the user, their membership, the criteria, and `now`.
pub fn evaluate_eligibility(
    user: &User,
    membership: &Membership,
    criteria: &FranchiseCriteria,
    now: Timestamp,
) -> Eligibility {
    // A franchise-barred account (a dev/content puppet) is never eligible, full
    // stop — no criterion, contribution, or age can lift the bar. Returned as the
    // sole unmet reason so callers/UI can say why.
    if user.is_franchise_barred {
        return Eligibility {
            unmet: vec![Unmet::Barred],
        };
    }

    let mut unmet = Vec::new();

    let account_age = user.account_age_days(now);
    if account_age < criteria.min_account_age_days {
        unmet.push(Unmet::AccountTooYoung {
            need_days: criteria.min_account_age_days,
            have_days: account_age,
        });
    }

    let member_age = membership.membership_age_days(now);
    if member_age < criteria.min_membership_days {
        unmet.push(Unmet::MembershipTooShort {
            need_days: criteria.min_membership_days,
            have_days: member_age,
        });
    }

    if membership.contribution < criteria.min_contribution {
        unmet.push(Unmet::InsufficientContribution {
            need: criteria.min_contribution,
            have: membership.contribution,
        });
    }

    if membership.is_sanctioned(now) {
        unmet.push(Unmet::Sanctioned);
    }

    Eligibility { unmet }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DemosId, UserId};

    const DAY: i64 = Timestamp::SECONDS_PER_DAY;

    fn user_aged(days: i64, now: Timestamp) -> User {
        User::new(UserId(1), "alice", Timestamp(now.0 - days * DAY))
    }

    fn member_aged(days: i64, contribution: i64, now: Timestamp) -> Membership {
        let mut m = Membership::joined(UserId(1), DemosId(1), Timestamp(now.0 - days * DAY));
        m.contribution = contribution;
        m
    }

    #[test]
    fn fully_qualified_member_is_eligible() {
        let now = Timestamp(100 * DAY);
        let e = evaluate_eligibility(
            &user_aged(40, now),
            &member_aged(20, 9, now),
            &FranchiseCriteria::platform_default(),
            now,
        );
        assert!(e.is_eligible(), "expected eligible, got {:?}", e.unmet);
    }

    #[test]
    fn franchise_barred_account_is_never_eligible() {
        let now = Timestamp(100 * DAY);
        // Otherwise fully qualified (old account, long membership, high contribution)
        // — the bar must override every satisfied criterion.
        let user = user_aged(40, now).barred();
        let e = evaluate_eligibility(
            &user,
            &member_aged(20, 9, now),
            &FranchiseCriteria::platform_default(),
            now,
        );
        assert!(!e.is_eligible());
        assert_eq!(e.unmet, vec![Unmet::Barred]);
    }

    #[test]
    fn fresh_flood_account_is_blocked_on_every_axis() {
        let now = Timestamp(100 * DAY);
        let e = evaluate_eligibility(
            &user_aged(1, now),
            &member_aged(1, 0, now),
            &FranchiseCriteria::platform_default(),
            now,
        );
        assert!(!e.is_eligible());
        assert_eq!(e.unmet.len(), 3); // young account, short membership, no contribution
    }

    #[test]
    fn sanction_alone_disqualifies() {
        let now = Timestamp(100 * DAY);
        let mut m = member_aged(20, 9, now);
        m.sanction_for(now, 30);
        let e = evaluate_eligibility(
            &user_aged(40, now),
            &m,
            &FranchiseCriteria::platform_default(),
            now,
        );
        assert_eq!(e.unmet, vec![Unmet::Sanctioned]);
    }
}
