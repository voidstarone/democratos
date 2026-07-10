/// One entry in the composer's community picker.
pub struct CommunityOption {
    pub slug: String,
    pub name: String,
    /// Marks the `<option selected>` (the community the user arrived from, or the
    /// first available).
    pub selected: bool,
}
