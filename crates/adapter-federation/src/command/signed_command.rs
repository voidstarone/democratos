use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use federation::NodeKeypair;

use crate::command::signing_payload::signing_payload;
use crate::Command;

/// A [`Command`] authenticated to the **forwarding node** by an Ed25519 signature
/// over its canonical JSON.
///
/// The command endpoint used to be protected only by the shared cluster bearer
/// token, so anyone holding that one symmetric secret could forward a write
/// naming an arbitrary `voter`/`user`/`juror`. Binding each command to the
/// forwarding node's control-plane-published key means the owner will only run a
/// command that a node it actually knows produced — a mere token-holder that is
/// not a keyed node can no longer inject writes, and every forwarded write is
/// attributable and non-repudiable.
///
/// (This authenticates the *node*, not the end user: within the fleet, nodes are
/// trusted to have authenticated their own users. Per-user capabilities would be
/// the next step and are out of scope here.)
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SignedCommand {
    /// The forwarding node id whose key signs the command.
    pub node: u16,
    /// Canonical JSON of the [`Command`].
    pub body: String,
    /// Unix seconds when this command was minted. The owner rejects a command
    /// outside a small freshness window, so a captured one can't be replayed later.
    pub issued_at: i64,
    /// A per-command unique value. The owner records recently-seen `(node, nonce)`
    /// pairs and rejects a repeat, so the *same* signed command can't be re-applied
    /// even within the freshness window.
    pub nonce: String,
    /// Hex Ed25519 signature over the canonical signing payload (node + issued_at +
    /// nonce + body), so none of those fields can be altered after signing.
    pub signature: String,
}

/// A process-unique, non-repeating nonce. It need not be cryptographically random —
/// it only has to be unique per command so the owner's replay cache can dedup it —
/// so a monotonic counter mixed with the process start time (unique per process)
/// suffices without pulling in an RNG dependency.
fn fresh_nonce() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let base = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{base:x}-{n:x}")
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl SignedCommand {
    /// Sign `cmd` with the forwarding node's keypair, stamping a fresh timestamp and
    /// nonce for anti-replay.
    pub fn sign(keypair: &NodeKeypair, cmd: &Command) -> Self {
        Self::sign_at(keypair, cmd, unix_now(), fresh_nonce())
    }

    /// Like [`sign`](Self::sign) but with an explicit timestamp and nonce — used by
    /// tests to exercise freshness and replay deterministically.
    pub fn sign_at(keypair: &NodeKeypair, cmd: &Command, issued_at: i64, nonce: String) -> Self {
        let body = serde_json::to_string(cmd).expect("Command serializes");
        let payload = signing_payload(keypair.node().0, issued_at, &nonce, &body);
        let signature = keypair.sign_hex(payload.as_bytes());
        Self {
            node: keypair.node().0,
            body,
            issued_at,
            nonce,
            signature,
        }
    }

    /// The inner command, parsed from the signed bytes. Callers must verify the
    /// signature (see [`verify_signed`](crate::command::verify_signed::verify_signed))
    /// before trusting the result.
    pub fn command(&self) -> Option<Command> {
        serde_json::from_str(&self.body).ok()
    }
}
