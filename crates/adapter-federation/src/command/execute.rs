use app::{
    AuthenticateError, CastJuryVoteError, CastVoteError, RegisterAccountError, Services,
    VotePostError,
};
use domain::{PostId, ProposalId, TrialId, UserId};

use crate::{Command, CommandOutcome, ForwardError};

/// Map a governance write's error to a routing failure: a store failure surfaces
/// as [`ForwardError::App`] (keeping the typed [`StoreError`](app::StoreError) so a
/// store outcome like `AlreadyVoted` survives the gateway), any domain rejection
/// as [`ForwardError::Rejected`] (a human-readable refusal the forwarder can
/// surface, which the owner-side HTTP handler maps to a 4xx).
fn cast_vote_forward(e: CastVoteError) -> ForwardError {
    let message = e.to_string();
    match e {
        CastVoteError::Store(s) => ForwardError::App(s),
        _ => ForwardError::Rejected(message),
    }
}

fn vote_post_forward(e: VotePostError) -> ForwardError {
    let message = e.to_string();
    match e {
        VotePostError::Store(s) => ForwardError::App(s),
        _ => ForwardError::Rejected(message),
    }
}

fn cast_jury_vote_forward(e: CastJuryVoteError) -> ForwardError {
    let message = e.to_string();
    match e {
        CastJuryVoteError::Store(s) => ForwardError::App(s),
        _ => ForwardError::Rejected(message),
    }
}

fn mint_account_forward(e: RegisterAccountError) -> ForwardError {
    let message = e.to_string();
    match e {
        RegisterAccountError::Store(s) => ForwardError::App(s),
        // A bad/duplicate handle or email, or a weak password, is a domain rejection
        // the forwarding node can surface to the user as a 4xx.
        _ => ForwardError::Rejected(message),
    }
}

fn authenticate_forward(e: AuthenticateError) -> ForwardError {
    let message = e.to_string();
    match e {
        AuthenticateError::Store(s) => ForwardError::App(s),
        // Invalid credentials is a merits refusal — surfaced to the user opaquely so
        // account existence never leaks (same message the local path returns).
        _ => ForwardError::Rejected(message),
    }
}

/// Run a command against the local, authoritative `Services` — the owner side.
/// Every domain rule is re-checked here; the forwarding node's claims are ignored.
pub async fn execute(services: &Services, cmd: &Command) -> Result<CommandOutcome, ForwardError> {
    match cmd {
        Command::CastVote {
            proposal,
            voter,
            aye,
            sig,
        } => {
            services
                .cast_vote(ProposalId(*proposal), UserId(*voter), *aye, sig.as_deref())
                .await
                .map_err(cast_vote_forward)?;
            Ok(CommandOutcome::Voted)
        }
        Command::VotePost {
            post,
            user,
            dir,
            sig,
        } => {
            let score = services
                .vote_post(PostId(*post), UserId(*user), *dir, sig.as_deref())
                .await
                .map_err(vote_post_forward)?;
            Ok(CommandOutcome::PostScore(score))
        }
        Command::CastJuryVote {
            trial,
            juror,
            guilty,
            sig,
        } => {
            let verdict = services
                .cast_jury_vote(TrialId(*trial), UserId(*juror), *guilty, sig.as_deref())
                .await
                .map_err(cast_jury_vote_forward)?;
            Ok(CommandOutcome::Verdict(verdict))
        }
        Command::MintAccount {
            handle,
            email,
            password,
        } => {
            // The issuer runs its own registration: validate the credential policy,
            // enforce handle/email uniqueness, hash the password, and mint the account
            // in THIS node's id namespace. Because this node is a trusted issuer, the
            // account clears `is_trusted_issuer` and replicates fleet-wide.
            let user = services
                .register_account(handle, email, password)
                .await
                .map_err(mint_account_forward)?;
            Ok(CommandOutcome::AccountMinted { id: user.id.0 })
        }
        Command::Authenticate { handle, password } => {
            // The home issuer holds the credentials (they never replicate), so it —
            // and only it — can verify. By handle, since that is what replicates.
            let user = services
                .authenticate_by_handle(handle, password)
                .await
                .map_err(authenticate_forward)?;
            Ok(CommandOutcome::Authenticated { id: user.id.0 })
        }
    }
}
