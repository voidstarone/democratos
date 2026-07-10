//! `vote` — drive every seeded voter to cast one ballot across the given node
//! web URLs, reporting latency percentiles, throughput, and an error breakdown.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::manifest::Manifest;
use crate::pct::pct;

#[derive(Default)]
struct Outcomes {
    ok: AtomicU64,
    // A 4xx: the owner rejected it (already-voted, fail-closed, not-a-voter…).
    rejected: AtomicU64,
    // Transport/5xx: the node or its owner link failed.
    errored: AtomicU64,
}

pub(crate) async fn vote(manifest_path: &str, nodes: &[String], concurrency: usize) -> Result<()> {
    anyhow::ensure!(!nodes.is_empty(), "pass at least one --nodes URL");
    let manifest: Manifest = serde_json::from_slice(&std::fs::read(manifest_path)?)?;
    let nodes: Vec<String> = nodes
        .iter()
        .map(|n| n.trim_end_matches('/').to_string())
        .collect();
    let pid = manifest.proposal_id;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let outcomes = Arc::new(Outcomes::default());
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::<f64>::with_capacity(
        manifest.voter_ids.len(),
    )));
    let sample_err = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));

    println!(
        "casting {} votes across {} node(s) at concurrency {}…",
        manifest.voter_ids.len(),
        nodes.len(),
        concurrency
    );
    let wall = Instant::now();
    let mut set = tokio::task::JoinSet::new();
    for (i, voter) in manifest.voter_ids.iter().copied().enumerate() {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let node = nodes[i % nodes.len()].clone();
        let client = client.clone();
        let outcomes = outcomes.clone();
        let latencies = latencies.clone();
        let sample_err = sample_err.clone();
        // Deterministic aye/nay mix (~60% aye) — no RNG needed.
        let choice = if i % 5 < 3 { "aye" } else { "nay" };
        set.spawn(async move {
            let _permit = permit;
            let url = format!("{node}/p/{pid}/vote");
            let t0 = Instant::now();
            let resp = client
                .post(&url)
                .header("Cookie", format!("uid={voter}"))
                .header("x-requested-with", "loadgen")
                .form(&[("choice", choice)])
                .send()
                .await;
            let ms = t0.elapsed().as_secs_f64() * 1000.0;
            latencies.lock().await.push(ms);
            match resp {
                Ok(r) if r.status().is_success() => {
                    outcomes.ok.fetch_add(1, Ordering::Relaxed);
                }
                Ok(r) => {
                    let code = r.status().as_u16();
                    let body = r.text().await.unwrap_or_default();
                    if (500..600).contains(&code) {
                        outcomes.errored.fetch_add(1, Ordering::Relaxed);
                    } else {
                        outcomes.rejected.fetch_add(1, Ordering::Relaxed);
                    }
                    let mut s = sample_err.lock().await;
                    if s.len() < 5 {
                        s.push(format!("HTTP {code}: {}", body.trim()));
                    }
                }
                Err(e) => {
                    outcomes.errored.fetch_add(1, Ordering::Relaxed);
                    let mut s = sample_err.lock().await;
                    if s.len() < 5 {
                        s.push(e.to_string());
                    }
                }
            }
        });
    }
    while set.join_next().await.is_some() {}
    let elapsed = wall.elapsed().as_secs_f64();

    let ok = outcomes.ok.load(Ordering::Relaxed);
    let rejected = outcomes.rejected.load(Ordering::Relaxed);
    let errored = outcomes.errored.load(Ordering::Relaxed);
    let total = ok + rejected + errored;
    let mut lat = Arc::try_unwrap(latencies).unwrap().into_inner();
    lat.sort_by(|a, b| a.partial_cmp(b).unwrap());

    println!("\n── vote results ──────────────────────────────");
    println!(
        "  requests      {total} in {elapsed:.2}s  ({:.0} req/s)",
        total as f64 / elapsed
    );
    println!("  accepted      {ok}");
    println!("  rejected(4xx) {rejected}   (already-voted / fail-closed / ineligible)");
    println!("  errored(5xx)  {errored}");
    println!(
        "  latency ms    p50 {:.1}  p90 {:.1}  p99 {:.1}  max {:.1}",
        pct(&lat, 0.50),
        pct(&lat, 0.90),
        pct(&lat, 0.99),
        lat.last().copied().unwrap_or(0.0)
    );
    let errs = sample_err.lock().await;
    if !errs.is_empty() {
        println!("  sample errors:");
        for e in errs.iter() {
            println!("    - {e}");
        }
    }
    Ok(())
}
