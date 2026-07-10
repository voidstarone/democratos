//! File a report against a post.

use axum::{
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use domain::{PostId, ReportReason, ReportTarget, RuleId};

use crate::handlers::current_user::current_user;
use crate::handlers::report_form::ReportForm;
use crate::handlers::render_error::render_error;
use crate::handlers::resolve_lang::resolve_lang;
use crate::AppState;

pub async fn report_post(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Form(form): Form<ReportForm>,
) -> Response {
    let enhanced = headers.contains_key("x-requested-with");
    let lang = resolve_lang(&headers);
    let Some(user) = current_user(&state, &headers).await else {
        return if enhanced {
            (StatusCode::UNAUTHORIZED, "sign in").into_response()
        } else {
            render_error(lang, None, "sign in first".to_string())
        };
    };
    let Ok(Some(post)) = state.services.posts.get(PostId(id)).await else {
        return if enhanced {
            (StatusCode::NOT_FOUND, "no such post").into_response()
        } else {
            render_error(lang, Some(user.handle), "no such post".to_string())
        };
    };

    // The reporter cites a rule from a dropdown. Resolve its id to the rule's
    // text so the moderation queue shows which rule was invoked, and record the
    // structured rule reference on the report itself.
    let rule_id = form
        .rule
        .as_deref()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(RuleId);
    let note = match rule_id {
        Some(rid) => state
            .services
            .list_rules(post.demos_id)
            .await
            .ok()
            .and_then(|rs| rs.into_iter().find(|r| r.id == rid).map(|r| r.text))
            .unwrap_or_default(),
        None => String::new(),
    };

    match state
        .services
        .file_report(
            user.id,
            post.demos_id,
            ReportTarget::Post(PostId(id)),
            ReportReason::RuleBreak { rule: rule_id },
            &note,
        )
        .await
    {
        Ok(_) => {
            if enhanced {
                return Json(serde_json::json!({ "ok": true })).into_response();
            }
            let slug = state
                .services
                .demoi
                .get(post.demos_id)
                .await
                .ok()
                .flatten()
                .map(|d| d.slug)
                .unwrap_or_default();
            Redirect::to(&format!("/d/{slug}/reports")).into_response()
        }
        Err(e) => {
            if enhanced {
                (StatusCode::BAD_REQUEST, e.to_string()).into_response()
            } else {
                render_error(lang, Some(user.handle), e.to_string())
            }
        }
    }
}
