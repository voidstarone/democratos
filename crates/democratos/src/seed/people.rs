//! The cast of seeded people.

use crate::seed::person::Person;

const fn p(handle: &'static str, fame: u8) -> Person {
    Person { handle, fame }
}

/// The cast. Founders lead the list of each community; the tail are low-fame
/// lurkers who stay below the politics posting threshold.
pub(crate) const PEOPLE: &[Person] = &[
    p("ansel", 3),   // photography founder
    p("graydon", 3), // rustlang founder
    p("hypatia", 3), // politics founder
    p("marie", 3),
    p("ada", 2),
    p("linus", 2),
    p("grace", 2),
    p("edsger", 1),
    p("alan", 1),
    p("katherine", 1),
    p("noether", 0),
    p("lovelace", 0),
    p("newbie", 0),
    p("lurker", 0),
];
