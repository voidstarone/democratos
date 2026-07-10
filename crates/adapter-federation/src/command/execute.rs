use app::{CastJuryVoteError, CastVoteError, Services, VotePostError};
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
    }
}
