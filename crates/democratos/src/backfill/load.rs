//! Parse a `democratos.json` snapshot into an [`adapter_store_postgres::ImportData`].

use std::path::Path;

use anyhow::{Context, Result};

use adapter_store_postgres::{ImportData, JuryBallotRow, PostVoteRow, VoteRow};

use crate::backfill::snapshot::Snapshot;

/// Parse `path` (a `democratos.json` snapshot) into an [`ImportData`].
pub fn load(path: impl AsRef<Path>) -> Result<ImportData> {
    let path = path.as_ref();
    let bytes =
        std::fs::read(path).with_context(|| format!("reading snapshot {}", path.display()))?;
    let snap: Snapshot = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing snapshot {}", path.display()))?;
    Ok(ImportData {
        users: snap.users,
        demoi: snap.demoi,
        memberships: snap.memberships,
        proposals: snap.proposals,
        votes: snap
            .votes
            .into_iter()
            .map(|v| VoteRow {
                proposal: v.proposal,
                voter: v.voter,
                aye: v.aye,
                weight: v.weight,
            })
            .collect(),
        post_votes: snap
            .post_votes
            .into_iter()
            .map(|v| PostVoteRow {
                post: v.post,
                user: v.user,
                up: v.up,
            })
            .collect(),
        rules: snap.rules,
        posts: snap.posts,
        comments: snap.comments,
        reports: snap.reports,
        trials: snap.trials,
        jury_ballots: snap
            .jury_ballots
            .into_iter()
            .map(|b| JuryBallotRow {
                trial: b.trial,
                juror: b.juror,
                guilty: b.guilty,
                weight: b.weight,
            })
            .collect(),
    })
}
