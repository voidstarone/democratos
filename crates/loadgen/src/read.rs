//! `read` — hammer a GET endpoint across nodes for a read-throughput number.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::pct::pct;

pub(crate) async fn read(
    nodes: &[String],
    path: &str,
    requests: u64,
    concurrency: usize,
) -> Result<()> {
    anyhow::ensure!(!nodes.is_empty(), "pass at least one --nodes URL");
    let nodes: Vec<String> = nodes
        .iter()
        .map(|n| n.trim_end_matches('/').to_string())
        .collect();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let ok = Arc::new(AtomicU64::new(0));
    let bad = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(tokio::sync::Mutex::new(Vec::<f64>::with_capacity(
        requests as usize,
    )));

    println!(
        "issuing {requests} GET {path} across {} node(s) at concurrency {concurrency}…",
        nodes.len()
    );
    let wall = Instant::now();
    let mut set = tokio::task::JoinSet::new();
    for i in 0..requests {
        let permit = sem.clone().acquire_owned().await.unwrap();
        let url = format!("{}{path}", nodes[(i as usize) % nodes.len()]);
        let client = client.clone();
        let (ok, bad, latencies) = (ok.clone(), bad.clone(), latencies.clone());
        set.spawn(async move {
            let _permit = permit;
            let t0 = Instant::now();
            let r = client.get(&url).send().await;
            latencies
                .lock()
                .await
                .push(t0.elapsed().as_secs_f64() * 1000.0);
            match r {
                Ok(r) if r.status().is_success() => {
                    ok.fetch_add(1, Ordering::Relaxed);
                }
                _ => {
                    bad.fetch_add(1, Ordering::Relaxed);
                }
            }
        });
    }
    while set.join_next().await.is_some() {}
    let elapsed = wall.elapsed().as_secs_f64();
    let mut lat = Arc::try_unwrap(latencies).unwrap().into_inner();
    lat.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("\n── read results ──────────────────────────────");
    println!(
        "  {} ok, {} bad in {:.2}s  ({:.0} req/s)",
        ok.load(Ordering::Relaxed),
        bad.load(Ordering::Relaxed),
        elapsed,
        requests as f64 / elapsed
    );
    println!(
        "  latency ms    p50 {:.1}  p90 {:.1}  p99 {:.1}  max {:.1}",
        pct(&lat, 0.50),
        pct(&lat, 0.90),
        pct(&lat, 0.99),
        lat.last().copied().unwrap_or(0.0)
    );
    Ok(())
}
