//! One seeded person.

/// One seeded person. `fame` (0–3) drives how much they post and how many
/// upvotes their content attracts, producing a deliberate popularity spread.
pub(crate) struct Person {
    pub(crate) handle: &'static str,
    pub(crate) fame: u8,
}
