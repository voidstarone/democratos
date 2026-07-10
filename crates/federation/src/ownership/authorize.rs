//! The gate before applying any replicated event: authenticate it and authorize it.

use domain::NodeId;

use crate::{
    binding_is_authoritative, event_scope, AuthError, ChangeEvent, EventScope, OwnershipRegistry,
    ParentDemos, SignedPart,
};

/// **The** gate before applying any replicated event: authenticate it *and*
/// authorize it. Returns the verified [`SignedPart`] only when all hold:
///
/// 1. the event is signed by the key the control plane has for its claimed node;
/// 2. the signing node is the **current owner** of the community the row
///    *actually* belongs to — derived from the **payload** via [`event_scope`]
///    (resolving a ballot's community from its parent through `parents`), never
///    the envelope's self-declared `demos`;
/// 3. the event's epoch is **not older** than that community's current epoch.
///
/// Binding the check to the payload-derived community is what stops a node that
/// legitimately owns *one* community from stamping its own `demos` onto an event
/// whose row belongs to *another* — which would let it forge memberships, votes,
/// or removals fleet-wide and defeat the anti-takeover design.
///
/// Global rows (user accounts) have no per-community owner; their sole authority
/// is the node that **minted** the id (its home node), so a compromised peer can
/// no longer overwrite an arbitrary account it did not create.
pub async fn authorize(
    registry: &dyn OwnershipRegistry,
    event: &ChangeEvent,
    parents: &dyn ParentDemos,
) -> Result<SignedPart, AuthError> {
    // Untrusted peek, only to learn which node's key to fetch. Its contents are
    // not trusted until the signature verifies below.
    let claimed = event.peek().map_err(AuthError::Fed)?;
    let key = registry
        .public_key(NodeId(claimed.node))
        .await
        .map_err(|e| AuthError::Registry(e.0))?
        .ok_or(AuthError::UnknownNode)?;

    // Authenticity: signature over the received bytes, by that node's key.
    let part = event.verify(&key).map_err(AuthError::Fed)?;

    // The authoritative community comes from the payload, not `part.demos`.
    let community = match event_scope(&part) {
        EventScope::Global { home } => {
            // A user account is authored only by its home (minting) node.
            if home != NodeId(part.node) {
                return Err(AuthError::WrongHome);
            }
            return Ok(part);
        }
        EventScope::Demos(d) => d,
        EventScope::ViaParent { kind, id } => {
            match parents
                .parent_demos(kind, id)
                .await
                .map_err(|e| AuthError::Registry(e.0))?
            {
                Some(d) => d,
                // Parent not replicated yet — treat like an unowned community so
                // the ordered puller retries once the parent arrives.
                None => return Err(AuthError::Unowned),
            }
        }
        EventScope::Indeterminate => return Err(AuthError::ScopeMismatch),
    };

    // Authorization: rightful owner of the *derived* community, non-stale epoch.
    let owner = registry
        .owner_of(community)
        .await
        .map_err(|e| AuthError::Registry(e.0))?
        .ok_or(AuthError::Unowned)?;
    if owner.owner != NodeId(part.node) {
        return Err(AuthError::NotOwner);
    }
    if part.epoch < owner.epoch {
        return Err(AuthError::StaleEpoch);
    }

    // Founder-signed home binding — the open-federation ownership anchor. Even
    // though `part.node` holds the ownership lease per the control plane, honour
    // the community's binding: the signer must be the founder-chosen home node or a
    // pre-authorized failover heir. A hostile node that seized the etcd holder key
    // it does not deserve is fenced HERE — it cannot produce a binding naming
    // itself without the community's secret key, so its events are rejected
    // fleet-wide. Communities with no binding (legacy / imported / pre-feature)
    // are unconstrained, exactly as before — this never breaks migration/import.
    if let Some(binding) = registry
        .home_binding(community)
        .await
        .map_err(|e| AuthError::Registry(e.0))?
    {
        let key = registry
            .community_key(community)
            .await
            .map_err(|e| AuthError::Registry(e.0))?;
        // Only an *authoritative* binding (one that verifies against the community
        // key) constrains ownership. A binding that doesn't verify — a poisoned one
        // written by a party with control-plane access, or one whose key is missing —
        // is treated as absent: it must NOT fence the honest owner's events (that
        // would be a fleet-wide DoS), and it cannot vouch for an impostor either.
        if binding_is_authoritative(&binding, key.as_ref()) && !binding.authorizes(part.node) {
            return Err(AuthError::NotBoundHome);
        }
    }

    Ok(part)
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    use crate::{
        ChangeOp, ClaimOutcome, CommunityKeypair, FedError, InMemoryRegistry, NoParents,
        NodeKeypair, ParentKind, RegistryError,
    };
    use domain::compose_id;

    #[tokio::test]
    async fn a_comment_authorizes_against_its_parent_posts_owner() {
        // The owner of the post's community may write a comment on it…
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        let comment = |kp: &NodeKeypair| {
            ChangeEvent::sign(
                kp,
                SignedPart {
                    node: 0,
                    epoch,
                    seq: 1,
                    demos: Some(7),
                    entity: "comments".into(),
                    op: ChangeOp::Upsert,
                    payload: serde_json::json!({ "id": 5, "post_id": 500, "author": 1, "body": "hi" }),
                },
            )
        };
        assert!(authorize(&reg, &comment(&owner), &FixedParent(Some(7)))
            .await
            .is_ok());
        // …but a node that does not own the post's community cannot forge one into it.
        let attacker = NodeKeypair::generate(NodeId(2));
        reg.publish_key(attacker.node(), &attacker.public().to_hex())
            .await
            .unwrap();
        assert_eq!(
            authorize(&reg, &comment(&attacker), &FixedParent(Some(7))).await,
            Err(AuthError::NotOwner)
        );
    }

    #[tokio::test]
    async fn a_lease_holder_not_named_in_the_home_binding_is_fenced_by_authorize() {
        let demos = 7u64;
        // A node seizes the ownership lease (e.g. via a compromised control plane,
        // or before the binding was published) and holds the current epoch.
        let seizer = NodeKeypair::generate(NodeId(2));
        let (reg, epoch) = registry_with_owner(&seizer, demos).await;
        // The founder-signed binding names node 1 as home (failover: 3) — NOT the
        // seizer. So even though it holds the lease, its events are fenced.
        let community = CommunityKeypair::generate(demos);
        reg.publish_community_key(demos, &community.public().to_hex(), "")
            .await
            .unwrap();
        reg.set_home_binding(&community.bind(1, vec![3], 1))
            .await
            .unwrap();
        let ev = ChangeEvent::sign(&seizer, part(0, Some(demos), epoch));
        assert_eq!(
            authorize(&reg, &ev, &NoParents).await,
            Err(AuthError::NotBoundHome)
        );
    }

    #[tokio::test]
    async fn the_bound_home_node_authorizes_normally() {
        let demos = 7u64;
        let home = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&home, demos).await;
        let community = CommunityKeypair::generate(demos);
        reg.publish_community_key(demos, &community.public().to_hex(), "")
            .await
            .unwrap();
        reg.set_home_binding(&community.bind(1, vec![3], 1))
            .await
            .unwrap();
        let ev = ChangeEvent::sign(&home, part(0, Some(demos), epoch));
        assert!(
            authorize(&reg, &ev, &NoParents).await.is_ok(),
            "the founder-chosen home node is authorized under its binding"
        );
    }

    /// An honest `posts` event: the envelope `demos` and the payload's `demos_id`
    /// agree (as a real owner's outbox always produces).
    fn part(node: u16, demos: Option<u64>, epoch: u64) -> SignedPart {
        SignedPart {
            node,
            epoch,
            seq: 1,
            demos,
            entity: "posts".into(),
            op: ChangeOp::Upsert,
            payload: match demos {
                Some(d) => serde_json::json!({ "id": 1, "demos_id": d }),
                None => serde_json::json!({ "id": 1 }),
            },
        }
    }

    /// A `ParentDemos` stub mapping a fixed parent id to a community, for the
    /// ballot-authorization tests.
    struct FixedParent(Option<u64>);
    #[async_trait]
    impl ParentDemos for FixedParent {
        async fn parent_demos(
            &self,
            _kind: ParentKind,
            _id: u64,
        ) -> Result<Option<u64>, RegistryError> {
            Ok(self.0)
        }
    }

    async fn registry_with_owner(kp: &NodeKeypair, demos: u64) -> (InMemoryRegistry, u64) {
        let reg = InMemoryRegistry::new();
        reg.publish_key(kp.node(), &kp.public().to_hex())
            .await
            .unwrap();
        let ClaimOutcome::Claimed { epoch } = reg.claim(demos, kp.node()).await.unwrap() else {
            panic!("first claim must succeed");
        };
        (reg, epoch)
    }

    #[tokio::test]
    async fn owner_event_at_current_epoch_is_authorized() {
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        let ev = ChangeEvent::sign(&owner, part(0, Some(7), epoch));
        assert!(authorize(&reg, &ev, &NoParents).await.is_ok());
    }

    #[tokio::test]
    async fn event_from_a_non_owner_is_rejected() {
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        // An impostor node, key published, correctly signs — but does not own d/7.
        let impostor = NodeKeypair::generate(NodeId(2));
        reg.publish_key(impostor.node(), &impostor.public().to_hex())
            .await
            .unwrap();
        let ev = ChangeEvent::sign(&impostor, part(0, Some(7), epoch));
        assert_eq!(
            authorize(&reg, &ev, &NoParents).await,
            Err(AuthError::NotOwner)
        );
    }

    #[tokio::test]
    async fn a_dethroned_owner_is_rejected_after_rehoming_to_another_node() {
        // Node 1 owns d/7 at epoch 1 and signs an event, then loses ownership.
        let old = NodeKeypair::generate(NodeId(1));
        let (reg, old_epoch) = registry_with_owner(&old, 7).await;
        let stale_event = ChangeEvent::sign(&old, part(0, Some(7), old_epoch));

        // Failover: node 1's lease lapses, node 2 claims → epoch bumps.
        reg.release(7, old.node()).await.unwrap();
        let new = NodeKeypair::generate(NodeId(2));
        reg.publish_key(new.node(), &new.public().to_hex())
            .await
            .unwrap();
        let ClaimOutcome::Claimed { epoch: new_epoch } = reg.claim(7, new.node()).await.unwrap()
        else {
            panic!("re-claim after release must succeed");
        };
        assert!(
            new_epoch > old_epoch,
            "re-claim must bump the epoch (fencing)"
        );

        // A different node now owns d/7, so node 1's event fails the owner check —
        // even though its signature is perfectly valid.
        assert_eq!(
            authorize(&reg, &stale_event, &NoParents).await,
            Err(AuthError::NotOwner)
        );
        assert!(stale_event.verify(&old.public()).is_ok());
    }

    #[tokio::test]
    async fn a_regained_owners_old_epoch_event_is_fenced_as_stale() {
        // The subtle case: the SAME node loses and then regains ownership. An
        // event it signed under the *old* epoch (e.g. delayed in flight) must not
        // be accepted under the new epoch — that is exactly what the epoch fence
        // is for, distinct from the different-node NotOwner case above.
        let node = NodeKeypair::generate(NodeId(1));
        let (reg, first_epoch) = registry_with_owner(&node, 7).await;
        let old_epoch_event = ChangeEvent::sign(&node, part(0, Some(7), first_epoch));

        reg.release(7, node.node()).await.unwrap();
        let ClaimOutcome::Claimed {
            epoch: second_epoch,
        } = reg.claim(7, node.node()).await.unwrap()
        else {
            panic!("re-claim must succeed");
        };
        assert!(
            second_epoch > first_epoch,
            "regain must still bump the epoch"
        );

        // Owner matches (node 1), but the event's epoch is stale → fenced.
        assert_eq!(
            authorize(&reg, &old_epoch_event, &NoParents).await,
            Err(AuthError::StaleEpoch)
        );
        // A fresh event at the new epoch is accepted.
        let fresh = ChangeEvent::sign(&node, part(0, Some(7), second_epoch));
        assert!(authorize(&reg, &fresh, &NoParents).await.is_ok());
    }

    #[tokio::test]
    async fn an_unpublished_node_key_is_rejected() {
        let reg = InMemoryRegistry::new();
        let ghost = NodeKeypair::generate(NodeId(9));
        let ev = ChangeEvent::sign(&ghost, part(0, Some(7), 1));
        assert_eq!(
            authorize(&reg, &ev, &NoParents).await,
            Err(AuthError::UnknownNode)
        );
    }

    #[tokio::test]
    async fn a_tampered_event_is_rejected_before_ownership_is_checked() {
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        let good = ChangeEvent::sign(&owner, part(0, Some(7), epoch));
        let forged = ChangeEvent::from_wire(
            good.body().replace("\"id\":1", "\"id\":666"),
            good.signature().to_string(),
        );
        assert_eq!(
            authorize(&reg, &forged, &NoParents).await,
            Err(AuthError::Fed(FedError::BadSignature))
        );
    }

    /// A `users` event whose account id was minted by `home_node` (its high bits).
    fn user_part(user_id: u64) -> SignedPart {
        SignedPart {
            node: 0, // stamped by sign()
            epoch: 0,
            seq: 1,
            demos: None,
            entity: "users".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": user_id, "handle": "alice" }),
        }
    }

    #[tokio::test]
    async fn a_global_user_event_is_authorized_by_its_home_node() {
        // A user account is global (no owning community); its authority is the
        // node that minted the id. Node 1 owns accounts it minted.
        let node = NodeKeypair::generate(NodeId(1));
        let reg = InMemoryRegistry::new();
        reg.publish_key(node.node(), &node.public().to_hex())
            .await
            .unwrap();
        let home_minted = compose_id(NodeId(1), 5); // high bits = node 1
        let ev = ChangeEvent::sign(&node, user_part(home_minted));
        assert!(authorize(&reg, &ev, &NoParents).await.is_ok());
    }

    #[tokio::test]
    async fn a_node_cannot_overwrite_a_user_it_did_not_mint() {
        // Regression: the cross-fleet account-takeover hole. Node 2 is a valid,
        // keyed federation member, but the account was minted by node 1 — node 2
        // must not be able to overwrite it (e.g. swap the password hash).
        let attacker = NodeKeypair::generate(NodeId(2));
        let reg = InMemoryRegistry::new();
        reg.publish_key(attacker.node(), &attacker.public().to_hex())
            .await
            .unwrap();
        let victims_account = compose_id(NodeId(1), 5); // minted by node 1
        let ev = ChangeEvent::sign(&attacker, user_part(victims_account));
        assert_eq!(
            authorize(&reg, &ev, &NoParents).await,
            Err(AuthError::WrongHome)
        );
    }

    #[tokio::test]
    async fn a_demos_scoped_row_smuggled_as_global_is_refused() {
        // A membership with no derivable community (payload missing `demos_id`)
        // must not slip through the global path — that was how a demos-scoped row
        // could dodge the owner check entirely.
        let node = NodeKeypair::generate(NodeId(1));
        let reg = InMemoryRegistry::new();
        reg.publish_key(node.node(), &node.public().to_hex())
            .await
            .unwrap();
        let part = SignedPart {
            node: 0,
            epoch: 0,
            seq: 1,
            demos: None,
            entity: "memberships".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "user_id": 1, "tier": "Voter" }),
        };
        let ev = ChangeEvent::sign(&node, part);
        assert_eq!(
            authorize(&reg, &ev, &NoParents).await,
            Err(AuthError::ScopeMismatch)
        );
    }

    #[tokio::test]
    async fn an_owner_cannot_forge_a_row_into_another_community() {
        // THE headline regression. Node 1 legitimately owns d/7. It hand-crafts an
        // event whose envelope claims d/7 (which it owns) but whose *payload* is a
        // membership row in d/8 (which it does not own) — trying to enfranchise
        // someone in a community it has no authority over. Binding the ownership
        // check to the payload-derived community rejects it.
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        // d/8 is owned by a different node.
        let other = NodeKeypair::generate(NodeId(2));
        reg.publish_key(other.node(), &other.public().to_hex())
            .await
            .unwrap();
        reg.claim(8, other.node()).await.unwrap();

        let forged = SignedPart {
            node: 0,
            epoch,
            seq: 1,
            demos: Some(7), // lies: claims the community it owns…
            entity: "memberships".into(),
            // …but the row belongs to d/8.
            payload: serde_json::json!({ "user_id": 99, "demos_id": 8, "tier": "Voter" }),
            op: ChangeOp::Upsert,
        };
        let ev = ChangeEvent::sign(&owner, forged);
        // Authorized against the payload's community (d/8), node 1 is not its owner.
        assert_eq!(
            authorize(&reg, &ev, &NoParents).await,
            Err(AuthError::NotOwner)
        );
    }

    #[tokio::test]
    async fn a_ballot_is_authorized_against_its_parents_community() {
        // A vote carries no demos_id; its community is resolved from the proposal
        // it attaches to. The owner of that community may cast it…
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        let vote = SignedPart {
            node: 0,
            epoch,
            seq: 1,
            demos: Some(7),
            entity: "votes".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "proposal_id": 500, "voter_id": 3, "aye": true }),
        };
        let ev = ChangeEvent::sign(&owner, vote.clone());
        // Parent proposal 500 lives in d/7 → owner authorized.
        assert!(authorize(&reg, &ev, &FixedParent(Some(7))).await.is_ok());

        // …but if the parent actually lives in d/8, the d/7 owner is not its owner.
        let reg2 = InMemoryRegistry::new();
        reg2.publish_key(owner.node(), &owner.public().to_hex())
            .await
            .unwrap();
        reg2.claim(7, owner.node()).await.unwrap();
        let other = NodeKeypair::generate(NodeId(2));
        reg2.publish_key(other.node(), &other.public().to_hex())
            .await
            .unwrap();
        reg2.claim(8, other.node()).await.unwrap();
        let ev2 = ChangeEvent::sign(&owner, vote);
        assert_eq!(
            authorize(&reg2, &ev2, &FixedParent(Some(8))).await,
            Err(AuthError::NotOwner)
        );
    }

    #[tokio::test]
    async fn a_ballot_whose_parent_is_not_yet_present_is_transiently_unowned() {
        // Parent not replicated locally yet → retryable, not a permanent reject.
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        let vote = SignedPart {
            node: 0,
            epoch,
            seq: 1,
            demos: Some(7),
            entity: "votes".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "proposal_id": 500, "voter_id": 3, "aye": true }),
        };
        let ev = ChangeEvent::sign(&owner, vote);
        assert_eq!(
            authorize(&reg, &ev, &FixedParent(None)).await,
            Err(AuthError::Unowned)
        );
    }

    #[tokio::test]
    async fn the_demoi_community_row_is_authorized_by_its_own_id() {
        // The `demoi` row is keyed by its own id (it has no demos_id); the current
        // owner of that community is its authority.
        let owner = NodeKeypair::generate(NodeId(1));
        let (reg, epoch) = registry_with_owner(&owner, 7).await;
        let demoi = SignedPart {
            node: 0,
            epoch,
            seq: 1,
            demos: None, // the outbox emits demoi with a NULL demos_id
            entity: "demoi".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": 7, "slug": "rust", "name": "Rustaceans" }),
        };
        let ev = ChangeEvent::sign(&owner, demoi);
        assert!(authorize(&reg, &ev, &NoParents).await.is_ok());

        // A node that does not own d/7 cannot rewrite its config.
        let attacker = NodeKeypair::generate(NodeId(2));
        reg.publish_key(attacker.node(), &attacker.public().to_hex())
            .await
            .unwrap();
        let forged = SignedPart {
            node: 0,
            epoch,
            seq: 2,
            demos: None,
            entity: "demoi".into(),
            op: ChangeOp::Upsert,
            payload: serde_json::json!({ "id": 7, "slug": "rust", "name": "pwned" }),
        };
        let ev2 = ChangeEvent::sign(&attacker, forged);
        assert_eq!(
            authorize(&reg, &ev2, &NoParents).await,
            Err(AuthError::NotOwner)
        );
    }
}
