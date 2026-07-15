//! Persistence for a community's rules.

use async_trait::async_trait;

use domain::{DemosId, Rule, RuleId, Timestamp};

use crate::Result;

#[async_trait]
pub trait RuleStore: Send + Sync {
    async fn create(
        &self,
        demos: DemosId,
        text: &str,
        sanction_days: u32,
        at: Timestamp,
    ) -> Result<Rule>;
    async fn get(&self, id: RuleId) -> Result<Option<Rule>>;
    async fn set_active(&self, id: RuleId, active: bool) -> Result<()>;
    async fn list_active(&self, demos: DemosId) -> Result<Vec<Rule>>;
}
