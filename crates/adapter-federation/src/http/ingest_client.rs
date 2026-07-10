use federation::ChangeEvent;

use crate::http::feed_error::FeedError;

/// Client used by an owner to push events to a standby and wait for the ack.
pub struct IngestClient {
    base_url: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl IngestClient {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token,
            http: reqwest::Client::new(),
        }
    }

    /// Push `events` (produced by `peer_node`) to this standby; returns how many
    /// it applied. An error means the standby did not durably ack.
    pub async fn push(&self, peer_node: i64, events: &[ChangeEvent]) -> Result<u64, FeedError> {
        let url = format!("{}/federation/ingest", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({ "peer_node": peer_node, "events": events });
        let mut req = self.http.post(url).json(&body);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| FeedError::Transport(e.to_string()))?;
        if !resp.status().is_success() {
            let code = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(FeedError::Transport(format!(
                "standby returned {code}: {body}"
            )));
        }
        resp.json()
            .await
            .map_err(|e| FeedError::Transport(e.to_string()))
    }
}
