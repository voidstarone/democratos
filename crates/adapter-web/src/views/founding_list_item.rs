/// A pending founding as it appears in a list (home page, etc.).
pub struct FoundingListItem {
    pub id: u64,
    pub slug: String,
    pub name: String,
    /// Co-signers gathered so far, and how many are required in total.
    pub signed: usize,
    pub required: usize,
}
