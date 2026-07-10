//! GUARDRAIL (malicious peer): a rogue node serving a forged feed.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};

use federation::ChangeEvent;

use crate::cli::Cli;
use crate::connect::connect;
use crate::current_epoch::current_epoch;
use crate::demoi_event::demoi_event;

pub(crate) async fn serve_rogue(cli: &Cli, demos: u64, bind: &str) -> Result<()> {
    use axum::{extract::State, routing::get, Json, Router};

    let (reg, kp) = connect(cli).await?;
    let epoch = current_epoch(&reg, demos).await;
    eprintln!(
        "== SERVE-ROGUE on {bind}: forging feed for d/{demos} as node {} (epoch {epoch}) ==",
        cli.node
    );

    // Two forged events an honest puller will pull and must reject (NotOwner — the
    // etcd owner of d/{demos} is the honest node, not this rogue).
    let forged: Arc<Vec<ChangeEvent>> = Arc::new(vec![
        demoi_event(&kp, demos, epoch, 1, "ROGUE-FEED-1"),
        demoi_event(&kp, demos, epoch, 2, "ROGUE-FEED-2"),
    ]);

    #[derive(serde::Deserialize)]
    struct Q {
        since: Option<i64>,
        #[allow(dead_code)]
        limit: Option<i64>,
    }

    async fn changes(
        State(events): State<Arc<Vec<ChangeEvent>>>,
        q: axum::extract::Query<Q>,
    ) -> Json<Vec<ChangeEvent>> {
        // Match the honest feed's contract: a bare JSON array, filtered by `since`
        // (seq index here) so the puller's cursor advances and it stops re-polling.
        let since = q.since.unwrap_or(0);
        let out: Vec<ChangeEvent> = events
            .iter()
            .enumerate()
            .filter(|(i, _)| (*i as i64 + 1) > since)
            .map(|(_, e)| e.clone())
            .collect();
        Json(out)
    }

    let app = Router::new()
        .route("/federation/changes", get(changes))
        .with_state(forged);
    let addr: SocketAddr = bind.parse().context("bad --bind addr")?;
    eprintln!("  rogue feed serving GET /federation/changes on {addr}; honest peers should reject.");
    println!("OUTCOME=SERVING");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
