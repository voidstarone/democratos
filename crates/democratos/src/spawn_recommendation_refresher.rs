//! Spawn the background recommendation refresher.

use app::Services;

/// Spawn the background recommendation refresher. The first tick fires
/// immediately so the feed is warm shortly after boot; thereafter it runs every
/// `refresh_secs`. A failed refresh is logged and retried next tick — it never
/// takes the server down.
pub(crate) fn spawn_recommendation_refresher(services: Services, refresh_secs: u64) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(refresh_secs.max(1)));
        loop {
            tick.tick().await; // immediate on the first iteration
            match services.refresh_recommendations().execute().await {
                Ok(true) => eprintln!("recommendations: model rebuilt"),
                Ok(false) => {} // unchanged since last build — nothing to do
                Err(e) => eprintln!("⚠ recommendations: refresh failed: {e}"),
            }
        }
    });
}
