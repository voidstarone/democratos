//! What the winning tag does to the content.

use crate::sensitive::sensitive_tag::SensitiveTag;

/// What a resolved case does to its target — the disposition the application layer
/// then carries out. Kept as a pure mapping in the domain so the *decision* is
/// testable independently of the storage side effects that execute it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReviewOutcome {
    /// A false flag: un-hide the content, unchanged. (`NotSensitive`.)
    Restore,
    /// Lawful adult content: un-hide but flag NSFW, so the existing blur/age gate
    /// applies. (`Porn`.)
    AgeGate,
    /// Take the content down platform-wide. `escalate` is set for CSAM, which must
    /// additionally be preserved and reported to the operator.
    Remove { escalate: bool },
}

/// Map a winning [`SensitiveTag`] to the disposition to apply.
pub fn outcome_for(tag: SensitiveTag) -> ReviewOutcome {
    match tag {
        SensitiveTag::NotSensitive => ReviewOutcome::Restore,
        SensitiveTag::Porn => ReviewOutcome::AgeGate,
        SensitiveTag::Csam => ReviewOutcome::Remove { escalate: true },
        SensitiveTag::Gore
        | SensitiveTag::SelfHarm
        | SensitiveTag::Spam
        | SensitiveTag::Other => ReviewOutcome::Remove { escalate: false },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csam_removes_and_escalates() {
        assert_eq!(
            outcome_for(SensitiveTag::Csam),
            ReviewOutcome::Remove { escalate: true }
        );
    }

    #[test]
    fn porn_age_gates_and_stays_up() {
        assert_eq!(outcome_for(SensitiveTag::Porn), ReviewOutcome::AgeGate);
    }

    #[test]
    fn false_flag_restores() {
        assert_eq!(outcome_for(SensitiveTag::NotSensitive), ReviewOutcome::Restore);
    }

    #[test]
    fn other_removals_do_not_escalate() {
        for t in [SensitiveTag::Gore, SensitiveTag::SelfHarm, SensitiveTag::Spam, SensitiveTag::Other] {
            assert_eq!(outcome_for(t), ReviewOutcome::Remove { escalate: false });
        }
    }
}
