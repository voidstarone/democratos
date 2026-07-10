//! How a member wants long feeds delivered.

use serde::{Deserialize, Serialize};

/// How a member wants long feeds delivered — a server-side account preference,
/// resolved on every feed render. It is only the *explicit* half of the decision:
/// a page still degrades to plain server-rendered pagination with no JavaScript,
/// and [`Auto`](Self::Auto) further defers to the browser's `prefers-reduced-motion`
/// hint at render time. See the web adapter's feed handlers.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedPaging {
    /// No explicit choice: lazy-load (infinite scroll) when the browser allows
    /// it, but fall back to paged links when JavaScript is off or the viewer has
    /// asked for reduced motion. The default.
    #[default]
    Auto,
    /// Always plain page-by-page navigation, even with JavaScript available.
    Pages,
    /// Always lazy-load when JavaScript is available, disregarding the
    /// reduced-motion hint.
    Lazy,
}
