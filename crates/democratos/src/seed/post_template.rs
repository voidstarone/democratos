//! A post template, cycled through per author.

/// Post templates, cycled through per author. Each is a title, a body, and
/// whether it carries a generated image attachment.
pub(crate) struct PostTemplate {
    pub(crate) title: &'static str,
    pub(crate) body: &'static str,
    pub(crate) with_image: bool,
    pub(crate) tags: &'static str,
}
