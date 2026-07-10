//! Integration tests for the etcd control plane, run against a real etcd.
//!
//! Gated on `TEST_ETCD_ENDPOINT` (unset → the tests no-op). To run:
//!
//! ```sh
//! docker run -d --name etcd -p 52379:2379 gcr.io/etcd-development/etcd:v3.5.16 \
//!   /usr/local/bin/etcd --advertise-client-urls http://0.0.0.0:2379 \
//!   --listen-client-urls http://0.0.0.0:2379
//! TEST_ETCD_ENDPOINT=http://127.0.0.1:52379 cargo test -p adapter-control-etcd
//! ```
//!
//! One sequential test: the scenarios share one etcd keyspace, so the test wipes
//! the `democratos/` prefix once up front and then runs each check in order.

use adapter_control_etcd::EtcdRegistry;
use domain::NodeId;
use etcd_client::{Client, DeleteOptions};
use federation::{ClaimOutcome, NodeKeypair, NodeLoad, OwnershipRegistry};

async fn endpoints() -> Option<Vec<String>> {
    let ep = std::env::var("TEST_ETCD_ENDPOINT").ok()?;
    // Wipe our keyspace for a deterministic run.
    let mut client = Client::connect([ep.clone()], None).await.expect("connect");
    client
        .delete("democratos/", Some(DeleteOptions::new().with_prefix()))
        .await
        .expect("wipe prefix");
    Some(vec![ep])
}

#[tokio::test]
async fn ownership_leases_epochs_and_registry() {
    let Some(eps) = endpoints().await else {
        eprintln!("skipping: TEST_ETCD_ENDPOINT not set");
        return;
    };

    let (a, b) = (NodeId(1), NodeId(2));
    let reg_a = EtcdRegistry::connect(&eps, 10, a).await.expect("connect A");
    let reg_b = EtcdRegistry::connect(&eps, 10, b).await.expect("connect B");

    // --- key registry ---
    let kp_a = NodeKeypair::generate(a);
    reg_a.publish_key(a, &kp_a.public().to_hex()).await.unwrap();
    let fetched = reg_b.public_key(a).await.unwrap().expect("key present");
    assert_eq!(fetched.to_hex(), kp_a.public().to_hex());
    assert!(reg_b.public_key(NodeId(77)).await.unwrap().is_none());

    // --- first claim wins, second is held off ---
    assert_eq!(
        reg_a.claim(7, a).await.unwrap(),
        ClaimOutcome::Claimed { epoch: 1 }
    );
    assert_eq!(
        reg_b.claim(7, b).await.unwrap(),
        ClaimOutcome::Held { by: a, epoch: 1 }
    );
    let owner = reg_b.owner_of(7).await.unwrap().unwrap();
    assert_eq!((owner.owner, owner.epoch), (a, 1));

    // --- release then reclaim bumps the epoch (fencing) ---
    reg_a.release(7, a).await.unwrap();
    assert!(
        reg_b.owner_of(7).await.unwrap().is_none(),
        "released → unowned"
    );
    assert_eq!(
        reg_b.claim(7, b).await.unwrap(),
        ClaimOutcome::Claimed { epoch: 2 },
        "reclaim must bump past the old epoch, never reset it"
    );
    let owner = reg_a.owner_of(7).await.unwrap().unwrap();
    assert_eq!((owner.owner, owner.epoch), (b, 2));

    // --- concurrent claim race: exactly one winner (anti-split-brain) ---
    // A fresh community both nodes rush at the same instant.
    let (ra, rb) = tokio::join!(reg_a.claim(42, a), reg_b.claim(42, b));
    let (ra, rb) = (ra.unwrap(), rb.unwrap());
    let claimed = [ra, rb]
        .iter()
        .filter(|o| matches!(o, ClaimOutcome::Claimed { .. }))
        .count();
    assert_eq!(
        claimed, 1,
        "exactly one node may own a community: {ra:?} {rb:?}"
    );

    // --- load reporting → live nodes ---
    reg_a
        .report_load(
            a,
            NodeLoad {
                hosted_communities: 3,
                requests_per_sec: 12.5,
            },
        )
        .await
        .unwrap();
    reg_b
        .report_load(
            b,
            NodeLoad {
                hosted_communities: 1,
                requests_per_sec: 2.0,
            },
        )
        .await
        .unwrap();
    let mut nodes = reg_a.live_nodes().await.unwrap();
    nodes.sort_by_key(|n| n.node.0);
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].node, a);
    assert_eq!(nodes[0].load.hosted_communities, 3);
    assert_eq!(nodes[1].load.hosted_communities, 1);
}
