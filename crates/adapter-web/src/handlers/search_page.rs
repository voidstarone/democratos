//! Search posts (and, site-wide, communities).

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    response::Response,
};

use app::{SearchResults, SearchScope};

use crate::handlers::current_user::current_user;
use crate::handlers::make_pager::make_pager;
use crate::handlers::page_of::page_of;
use crate::handlers::paginate::paginate;
use crate::handlers::paging_mode::paging_mode;
use crate::handlers::post_row::post_row;
use crate::handlers::render::render;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::search_query::SearchQuery;
use crate::handlers::wants_fragment::wants_fragment;
use crate::views::demos_list_item::DemosListItem;
use crate::views::search_fragment_view::SearchFragmentView;
use crate::views::search_view::SearchView;
use crate::AppState;

/// Percent-encode a query-parameter value, encoding everything outside the RFC
/// 3986 unreserved set. Keeps a search query with spaces/`&`/`#` safe inside a
/// pager link without pulling in a URL-encoding dependency.
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for &b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Search posts (and, site-wide, communities). Scope is `all` or a community
/// slug; a plain GET form, so it works with no JavaScript.
pub async fn search_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<SearchQuery>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    let page = page_of(query.page);
    let mode = paging_mode(user.as_ref());

    // Resolve the scope: a non-empty, non-"all" value is a community slug.
    let (scope, scope_slug) = if query.scope.is_empty() || query.scope == "all" {
        (SearchScope::All, None)
    } else {
        match state.services.demoi.by_slug(&query.scope).await {
            Ok(Some(d)) => (SearchScope::Demos(d.id), Some(d.slug)),
            _ => (SearchScope::All, None),
        }
    };

    let tag = query.tag.as_deref().filter(|t| !t.is_empty());
    let results: SearchResults = state
        .services
        .search(&query.q, scope, tag)
        .await
        .unwrap_or_default();

    // Search results span communities the viewer may not belong to, so scores
    // are read-only here (no vote arrows).
    let (window, has_next) = paginate(results.posts, page);
    let mut posts = Vec::new();
    for p in &window {
        posts.push(post_row(&state, p, None, false, None).await);
    }

    // Preserve the query across pages, percent-encoding user text.
    let mut base = format!("/search?q={}", enc(&query.q));
    if !query.scope.is_empty() {
        base.push_str(&format!("&scope={}", enc(&query.scope)));
    }
    if let Some(tg) = tag {
        base.push_str(&format!("&tag={}", enc(tg)));
    }
    base.push('&');
    let pager = make_pager(&base, page, has_next, mode);

    if wants_fragment(&headers) {
        return render(SearchFragmentView {
            t: lang.strings(),
            posts,
            pager,
        });
    }

    // Matching communities are a first-page header, not part of the post feed.
    let communities = if page == 1 {
        results
            .communities
            .into_iter()
            .map(|d| DemosListItem {
                slug: d.slug,
                name: d.name,
                phase: String::new(),
                voters: 0,
            })
            .collect()
    } else {
        Vec::new()
    };

    render(SearchView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: user.map(|u| u.handle),
        query: query.q,
        scope_slug,
        tag: tag.map(str::to_string),
        communities,
        posts,
        pager,
    })
}
