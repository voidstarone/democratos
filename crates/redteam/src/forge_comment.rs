//! PROBE HIGH-1: what does an honest node do with a NULL-scoped comment?

use anyhow::Result;

use federation::{ChangeEvent, ChangeOp, SignedPart};

use crate::cli::Cli;
use crate::connect::connect;
use crate::push::push;

pub(crate) async fn forge_comment(cli: &Cli, post: u64, feed: &str) -> Result<()> {
    let (_reg, kp) = connect(cli).await?;
    eprintln!("== FORGE-COMMENT post {post} → {feed} (HIGH-1 probe) ==");
    // Mirror the real outbox emission: a `comments` row carries NO demos_id (the
    // table has no such column), so the signed payload has none either.
    let ev = ChangeEvent::sign(
        &kp,
        SignedPart {
            node: 0,
            epoch: 1,
            seq: 1,
            demos: None, // exactly how the outbox emits comment events today
            entity: "comments".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({
                "id": 999_001u64, "post_id": post, "author": 1,
                "body": "FORGED CROSS-COMMUNITY COMMENT", "removed": false
            }),
        },
    );
    let applied = push(feed, cli.token.clone(), cli.node as i64, std::slice::from_ref(&ev)).await?;
    eprintln!(
        "  honest node applied {applied} of 1 comment events \
         (applied>0 = forgery; 0 = rejected/ScopeMismatch — a liveness bug, not forgery)"
    );
    println!("OUTCOME=APPLIED:{applied}");
    Ok(())
}
