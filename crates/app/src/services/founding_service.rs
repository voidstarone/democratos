//! Founding use-cases: the direct `found_demos` shortcut and the petition-driven
//! path (open a petition, gather sign-offs, found for real at quorum). Owns the
//! founding-facing ports and calls [`AccountService`](super::account_service::AccountService)
//! (its one peer) to enforce the franchise bar on founders and co-signers.

use std::sync::Arc;

use domain::{slugify, Demos, FoundingId, FoundingPetition, Membership, Tier, UserId};

use crate::{
    Clock, DemosStore, FoundDemosError, FoundingStore, MembershipStore, Result, SignFoundingError,
    StartFoundingError, StoreError,
};

use super::account_service::AccountService;

/// Founding use-cases, over just the founding-facing ports plus the account peer.
#[derive(Clone)]
pub struct FoundingService {
    foundings: Arc<dyn FoundingStore>,
    demoi: Arc<dyn DemosStore>,
    memberships: Arc<dyn MembershipStore>,
    clock: Arc<dyn Clock>,
    account: Arc<AccountService>,
}

impl FoundingService {
    pub fn new(
        foundings: Arc<dyn FoundingStore>,
        demoi: Arc<dyn DemosStore>,
        memberships: Arc<dyn MembershipStore>,
        clock: Arc<dyn Clock>,
        account: Arc<AccountService>,
    ) -> Self {
        Self {
            foundings,
            demoi,
            memberships,
            clock,
            account,
        }
    }

    pub async fn found_demos(
        &self,
        founder: UserId,
        slug: &str,
        name: &str,
    ) -> Result<Demos, FoundDemosError> {
        self.found_demos_tagged(founder, slug, name, Vec::new()).await
    }

    /// [`found_demos`](Self::found_demos) with founder-chosen topic `tags` (already
    /// normalized). The petition-driven path founds through here so a community's
    /// tags — captured when the petition opened — land on the demos it becomes.
    async fn found_demos_tagged(
        &self,
        founder: UserId,
        slug: &str,
        name: &str,
        tags: Vec<String>,
    ) -> Result<Demos, FoundDemosError> {
        self.account.ensure_not_barred(founder).await?;
        if self.demoi.by_slug(slug).await?.is_some() {
            return Err(StoreError::AlreadyExists.into());
        }
        let now = self.clock.now();
        let demos = self.demoi.create(slug, name, founder, tags, now).await?;

        let mut m = Membership::joined(founder, demos.id, now);
        m.tier = Tier::Voter;
        m.enfranchised_at = Some(now);
        self.memberships.upsert(m).await?;

        Ok(demos)
    }

    /// Open a founding petition. A demos is no longer created outright: the
    /// founder proposes a name (its slug is derived here, the single source of
    /// truth) and must gather [`domain::SIGN_OFFS_REQUIRED`] co-signers before it
    /// becomes real — see [`sign_founding`](Self::sign_founding). Rejected if the
    /// slug is already taken by a live demos or another open petition.
    pub async fn start_founding(
        &self,
        founder: UserId,
        name: &str,
    ) -> Result<FoundingPetition, StartFoundingError> {
        self.start_founding_tagged(founder, name, Vec::new()).await
    }

    /// [`start_founding`](Self::start_founding) with founder-chosen topic `tags`
    /// (already normalized by the caller, as post tags are). They are carried on
    /// the petition until the community is founded, then applied to the demos.
    pub async fn start_founding_tagged(
        &self,
        founder: UserId,
        name: &str,
        tags: Vec<String>,
    ) -> Result<FoundingPetition, StartFoundingError> {
        self.account.ensure_not_barred(founder).await?;
        let name = name.trim();
        let slug = slugify(name);
        if slug.is_empty() {
            return Err(StartFoundingError::Rejected(
                "a community name needs at least one letter or number".into(),
            ));
        }
        if self.demoi.by_slug(&slug).await?.is_some() {
            return Err(StoreError::AlreadyExists.into());
        }
        if self.foundings.list().await?.iter().any(|p| p.slug == slug) {
            return Err(StoreError::AlreadyExists.into());
        }
        Ok(self
            .foundings
            .create(&slug, name, founder, tags, self.clock.now())
            .await?)
    }

    pub async fn founding(&self, id: FoundingId) -> Result<Option<FoundingPetition>> {
        self.foundings.get(id).await
    }

    /// Every founding still gathering sign-offs, newest first.
    pub async fn pending_foundings(&self) -> Result<Vec<FoundingPetition>> {
        self.foundings.list().await
    }

    /// Sign off on a pending founding. Idempotent per user; the founder cannot
    /// sign their own. When the final required co-signer commits, the demos is
    /// founded for real — the founder **and every co-signer** become founding
    /// voters, so a community is born with all ten already enfranchised (which
    /// lands it past the Seed phase) — and the petition is cleared. Returns the
    /// founded [`Demos`] when quorum was reached this call, otherwise `None`.
    pub async fn sign_founding(
        &self,
        id: FoundingId,
        user: UserId,
    ) -> Result<Option<Demos>, SignFoundingError> {
        let petition = self.foundings.get(id).await?.ok_or(StoreError::NotFound)?;
        if user == petition.founder {
            return Err(SignFoundingError::Rejected(
                "the founder already backs this founding".into(),
            ));
        }
        // Co-signing enfranchises the signer when quorum lands, so a barred puppet
        // must not be able to sign its way into the franchise.
        self.account.ensure_not_barred(user).await?;
        let petition = self.foundings.sign(id, user).await?;
        if !petition.is_ready() {
            return Ok(None);
        }

        // Quorum reached — found the demos (which enfranchises the founder), then
        // enfranchise every co-signer as a founding voter too. The founder's tags,
        // captured when the petition opened, are applied to the new community here.
        let demos = self
            .found_demos_tagged(
                petition.founder,
                &petition.slug,
                &petition.name,
                petition.tags.clone(),
            )
            .await?;
        let now = self.clock.now();
        for signer in &petition.sign_offs {
            let mut m = Membership::joined(*signer, demos.id, now);
            m.tier = Tier::Voter;
            m.enfranchised_at = Some(now);
            self.memberships.upsert(m).await?;
        }
        self.foundings.delete(id).await?;
        Ok(Some(demos))
    }
}
