//! The single-post page handler (plus its view builder and comment flattener).

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
};
use domain::{build_comment_tree, CommentNode, PostId, User};

use crate::handlers::current_user::current_user;
use crate::handlers::handle_of::handle_of;
use crate::handlers::render::render;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::handlers::viewer_blocks::viewer_blocks;
use crate::i18n::lang::Lang;
use crate::views::comment_row::CommentRow;
use crate::views::media_item::MediaItem;
use crate::views::post_view::PostView;
use crate::views::rule_view::RuleView;
use crate::AppState;

pub async fn post_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Response {
    let lang = resolve_lang(&headers);
    let user = current_user(&state, &headers).await;
    // Content flagged pending sensitive-content review is hidden from everyone but
    // opted-in reviewers (who act on it through the /review console).
    if let Ok(Some(post)) = state.services.posts.get(PostId(id)).await {
        let is_reviewer = user.as_ref().map(|u| u.is_sensitive_reviewer).unwrap_or(false);
        if post.pending_review && !is_reviewer {
            return render_error(
                lang,
                user.map(|u| u.handle),
                lang.strings().pending_review_notice.to_string(),
            );
        }
    }
    match build_post_view(&state, lang, PostId(id), user.as_ref()).await {
        Ok(view) => render(view),
        Err(e) => render_error(lang, user.map(|u| u.handle), e.to_string()),
    }
}

async fn build_post_view(
    state: &AppState,
    lang: Lang,
    post_id: PostId,
    viewer: Option<&User>,
) -> app::Result<PostView> {
    let post = state
        .services
        .posts
        .get(post_id)
        .await?
        .ok_or(app::StoreError::NotFound)?;
    let demos = state
        .services
        .demoi
        .get(post.demos_id)
        .await?
        .ok_or(app::StoreError::NotFound)?;

    let membership = match viewer {
        Some(u) => state.services.memberships.get(u.id, post.demos_id).await?,
        None => None,
    };
    let viewer_can_post = membership
        .as_ref()
        .map(|m| !m.is_sanctioned(state.services.clock.now()))
        .unwrap_or(false);
    let viewer_is_voter = membership.as_ref().map(|m| m.is_voter()).unwrap_or(false);

    let score = state.services.post_score(post_id).await?;
    let viewer_vote = match viewer {
        Some(u) => state.services.user_post_vote(post_id, u.id).await?,
        None => None,
    };

    // The template renders the body (if any) followed by every attachment.
    let kind_label = post.kind_label().to_string();
    let media: Vec<MediaItem> = post
        .media
        .iter()
        .map(|m| MediaItem {
            url: m.url.clone(),
            is_video: m.is_video,
            caption: m.caption.clone(),
        })
        .collect();

    // The community's rules populate the report form's rule-selection dropdown.
    let rules = state
        .services
        .list_rules(post.demos_id)
        .await?
        .into_iter()
        .map(|r| RuleView {
            id: r.id.0,
            text: r.text,
            ban_term: crate::i18n::rule_ban_term::rule_ban_term(lang, r.sanction_days),
        })
        .collect();

    let tree = build_comment_tree(state.services.comments_for(post_id).await?);
    let mut flat = Vec::new();
    flatten_comments(&tree, 0, &mut flat);
    let viewer_id = viewer.as_ref().map(|u| u.id);
    // A blocked author's comment is kept in place (so replies below it stay
    // threaded) but its body is withheld and voting suppressed.
    let blocked = viewer_blocks(state, viewer_id).await;
    let mut comments = Vec::new();
    for (c, depth) in flat {
        let is_blocked = blocked.contains(&c.author);
        let score = state.services.comment_score(c.id).await.unwrap_or(0);
        let viewer_vote = match viewer_id {
            Some(u) => state
                .services
                .user_comment_vote(c.id, u)
                .await
                .unwrap_or(None),
            None => None,
        };
        comments.push(CommentRow {
            id: c.id.0,
            author: handle_of(state, c.author).await,
            body: c.body,
            depth,
            removed: c.removed,
            is_blocked,
            score,
            voted_up: viewer_vote == Some(true),
            voted_down: viewer_vote == Some(false),
            votable: viewer_can_post && !is_blocked,
        });
    }

    Ok(PostView {
        t: lang.strings(),
        lang: lang.code(),
        current_user: viewer.map(|u| u.handle.clone()),
        viewer_is_voter,
        demos_slug: demos.slug,
        id: post.id.0,
        title: post.title,
        author: handle_of(state, post.author).await,
        kind_label,
        body: post.body,
        media,
        tags: post.tags,
        score,
        voted_up: viewer_vote == Some(true),
        voted_down: viewer_vote == Some(false),
        votable: viewer_can_post,
        removed: post.removed,
        is_nsfw: post.is_nsfw,
        viewer_can_post,
        rules,
        comments,
    })
}

/// Depth-first flatten of the comment tree into (comment, indent-depth) rows.
fn flatten_comments(nodes: &[CommentNode], depth: usize, out: &mut Vec<(domain::Comment, usize)>) {
    for n in nodes {
        out.push((n.comment.clone(), depth));
        flatten_comments(&n.children, depth + 1, out);
    }
}
