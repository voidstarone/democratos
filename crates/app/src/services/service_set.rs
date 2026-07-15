//! The dependency-injection container: one built instance of every per-area
//! service, ready to inject individually. This is a wiring bundle, not a god
//! object — it holds no logic and no ports, only the assembled services so a
//! composition root can hand each consumer exactly the service(s) it needs.
//!
//! Built from a fully-wired [`Services`] via [`ServiceSet::from_services`] while
//! the strangler migration is in flight; once every call site injects services
//! directly, `Services` and its builders go away and this is constructed from the
//! store ports itself.

use std::sync::Arc;

use super::account_service::AccountService;
use super::blocking_service::BlockingService;
use super::content_service::ContentService;
use super::feed_service::FeedService;
use super::founding_service::FoundingService;
use super::governance_service::GovernanceService;
use super::invite_service::InviteService;
use super::membership_service::MembershipService;
use super::metrics_service::MetricsService;
use super::moderation_service::ModerationService;
use super::notification_service::NotificationService;
use super::profile_service::ProfileService;
use super::search_service::SearchService;
use super::sensitive_review_service::SensitiveReviewService;
use super::services::Services;

/// One shared instance of each per-area service. Cheap to clone (every field is
/// an `Arc`), so a handler can hold just the services it uses.
#[derive(Clone)]
pub struct ServiceSet {
    pub accounts: Arc<AccountService>,
    pub blocking: Arc<BlockingService>,
    pub profile: Arc<ProfileService>,
    pub search: Arc<SearchService>,
    pub notifications: Arc<NotificationService>,
    pub metrics: Arc<MetricsService>,
    pub invites: Arc<InviteService>,
    pub sensitive: Arc<SensitiveReviewService>,
    pub membership: Arc<MembershipService>,
    pub founding: Arc<FoundingService>,
    pub moderation: Arc<ModerationService>,
    pub governance: Arc<GovernanceService>,
    pub content: Arc<ContentService>,
    pub feed: Arc<FeedService>,
}

impl ServiceSet {
    /// Assemble every service from an already-wired [`Services`]. Each service is
    /// stateless (it holds only `Arc<dyn …Store>` port handles), so building peers
    /// per service rather than pointer-sharing them is behaviourally identical.
    pub fn from_services(s: &Services) -> Self {
        Self {
            accounts: Arc::new(s.account_service()),
            blocking: Arc::new(s.blocking_service()),
            profile: Arc::new(s.profile_service()),
            search: Arc::new(s.search_service()),
            notifications: Arc::new(s.notification_service()),
            metrics: Arc::new(s.metrics_service()),
            invites: Arc::new(s.invite_service()),
            sensitive: Arc::new(s.sensitive_review_service()),
            membership: Arc::new(s.membership_service()),
            founding: Arc::new(s.founding_service()),
            moderation: Arc::new(s.moderation_service()),
            governance: Arc::new(s.governance_service()),
            content: Arc::new(s.content_service()),
            feed: Arc::new(s.feed_service()),
        }
    }
}
