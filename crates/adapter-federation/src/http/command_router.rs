use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};

use crate::command::signed_command::SignedCommand;
use crate::command::verify_signed::verify_signed;
use crate::http::bearer_ok::bearer_ok;
use crate::http::command_state::CommandState;
use crate::{execute, CommandOutcome};

fn check_command_token(state: &CommandState, headers: &HeaderMap) -> Result<(), StatusCode> {
    bearer_ok(state.token.as_deref(), headers)
        .then_some(())
        .ok_or(StatusCode::UNAUTHORIZED)
}

async fn command_handler(
    State(state): State<CommandState>,
    headers: HeaderMap,
    Json(signed): Json<SignedCommand>,
) -> Result<Json<CommandOutcome>, (StatusCode, String)> {
    check_command_token(&state, &headers).map_err(|c| (c, "unauthorized".into()))?;
    // Authenticate the forwarding node: the command must carry a valid Ed25519
    // signature by a control-plane-published node key. This is what keeps a bare
    // token-holder (or a node whose key the fleet doesn't know) from forging a
    // write naming an arbitrary user.
    let cmd = verify_signed(state.registry.as_ref(), &signed)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    // Anti-replay: a valid signature proves *who* signed but not that this is a
    // fresh submission. Reject a command outside the freshness window or one whose
    // nonce we have already applied, so a captured command can't be re-played.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    state
        .replay_guard
        .admit(signed.node, &signed.nonce, signed.issued_at, now)
        .await
        .map_err(|e| (StatusCode::UNAUTHORIZED, e.to_string()))?;
    // Anti-abuse: a delegated mint is rate-limited per requesting node, so an
    // authenticated-but-abusive node can't flood the federation with accounts.
    if matches!(cmd, crate::Command::MintAccount { .. })
        && !state.mint_rate_limiter.admit(signed.node, now).await
    {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "account-minting rate limit exceeded for this node".into(),
        ));
    }
    // Brute-force guard on delegated login: cap verification attempts per target
    // handle, so a node can't grind an account's password by forwarding guesses.
    if let crate::Command::Authenticate { handle, .. } = &cmd {
        if !state.auth_rate_limiter.admit(handle, signed.node, now).await {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "login rate limit exceeded for this account".into(),
            ));
        }
    }
    // Minting an account claims its handle in the fleet-wide namespace FIRST, via an
    // atomic control-plane reservation, so two trusted issuers can never both mint the
    // same handle (which would let a colliding account impersonate another, since
    // login resolves by handle). If it is held by another issuer, refuse; if the
    // subsequent creation fails, release it so a rejected sign-up doesn't strand it.
    if let crate::Command::MintAccount { handle, .. } = &cmd {
        let reserved = state
            .registry
            .reserve_handle(handle.trim(), state.node)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.0))?;
        if !reserved {
            return Err((StatusCode::CONFLICT, "that handle is taken".into()));
        }
    }
    // Execute the real use-case: every domain rule is re-validated here on the
    // owner. A domain rejection is a 4xx (the forwarder can surface it); an
    // infrastructure failure is a 5xx.
    let outcome = execute(&state.services, &cmd).await;
    if let (crate::Command::MintAccount { handle, .. }, Err(_)) = (&cmd, &outcome) {
        // Creation failed after we reserved — give the handle back.
        let _ = state.registry.release_handle(handle.trim(), state.node).await;
    }
    match outcome {
        Ok(outcome) => Ok(Json(outcome)),
        // An infrastructure/store failure is a 5xx (surfaced with the raw message,
        // as before); any domain rejection — including a typed store outcome such as
        // `AlreadyVoted` — is a 4xx the forwarder can surface.
        Err(crate::ForwardError::App(app::StoreError::Store(e))) => {
            Err((StatusCode::INTERNAL_SERVER_ERROR, e))
        }
        Err(e) => Err((StatusCode::UNPROCESSABLE_ENTITY, e.to_string())),
    }
}

/// The command router — mount alongside the feed on the node-only address.
pub fn command_router(state: CommandState) -> Router {
    Router::new()
        .route("/federation/command", post(command_handler))
        .with_state(state)
}
