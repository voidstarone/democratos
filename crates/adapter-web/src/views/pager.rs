/// Pagination controls for one feed render. The same struct drives both the
/// no-JS fallback (plain page links) and the JS lazy-loader, which reads
/// [`next_href`](Self::next_href) and [`mode`](Self::mode) off the rendered link.
pub struct Pager {
    /// The effective delivery mode for this render: `"pages"` (always page links),
    /// `"lazy"` (always lazy-load with JS), or `"auto"` (lazy unless the browser
    /// asks for reduced motion). Resolved from the viewer's account preference.
    pub mode: &'static str,
    /// Link to the previous page, shown only in `"pages"` mode. `None` on page 1.
    pub prev_href: Option<String>,
    /// Link to the next page. `None` when this is the last page. Always a real URL
    /// so a no-JS click loads the next page as its own document.
    pub next_href: Option<String>,
}
