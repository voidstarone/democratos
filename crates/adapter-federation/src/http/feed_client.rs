use federation::ChangeEvent;

use crate::http::feed_error::FeedError;

/// A client for one peer's change feed.
pub struct FeedClient {
    base_url: String,
    token: Option<String>,
    http: reqwest::Client,
}

impl FeedClient {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        Self {
            base_url: base_url.into(),
            token,
            http: reqwest::Client::new(),
        }
    }

    /// Fetch events after `since` (oldest first).
    pub async fn changes_since(
        &self,
        since: i64,
        limit: i64,
    ) -> Result<Vec<ChangeEvent>, FeedError> {
        let url = format!(
            "{}/federation/changes?since={since}&limit={limit}",
            self.base_url.trim_end_matches('/')
        );
        let mut req = self.http.get(url);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| FeedError::Transport(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(FeedError::Status(resp.status().as_u16()));
        }
        resp.json()
            .await
            .map_err(|e| FeedError::Transport(e.to_string()))
    }
}
