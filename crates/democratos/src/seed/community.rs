//! A seeded community definition.

use domain::PostingPolicy;

/// A seeded community: slug, display name, the handle that founds it, and the
/// posting policy it ends up with *after* content is loaded.
pub(crate) struct Community {
    pub(crate) slug: &'static str,
    pub(crate) name: &'static str,
    pub(crate) founder: &'static str,
    /// Applied last, once posts and votes exist — so a gated community still gets
    /// seed content, then locks down going forward.
    pub(crate) final_policy: PostingPolicy,
}
