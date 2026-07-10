# Byzantine (majority-compromised) federation harness

A fully **dockerized**, **platform-independent** test harness that stands up a real
Democratos federation and attacks it from a **majority-compromised** position, using
genuine cryptography. It answers one question:

> When an attacker controls the **majority** of nodes, the shared cluster token, and
> the etcd control plane, can they seize, forge into, or rewrite an **honest,
> founder-bound community** owned by the honest minority?

The design thesis is that they cannot — because in this open federation a community's
authority is its **founder key** (held by the honest home node), not node-count or
etcd-write. This harness proves the guardrails empirically and exercises the
known-open holes from the security audit as documented `xfail`s.

**Validated (2026-07-10):** ran at **2 honest / 3 compromised** (5 nodes) and **2
honest / 5 compromised** (7 nodes) — the majority buys the attacker nothing against a
bound community. After the follow-up fixes (FED-1/FED-3/HIGH-1), the harness reports
**8/8 guardrails pass, 0 known-open**: the three previously-open findings are now
closed and confirmed here end-to-end.

## Why it's platform-independent

- Orchestrated with the **plain `docker` CLI** — no `docker compose` plugin (which
  isn't installed here) and no `docker-compose`. Works identically on colima/arm64
  (macOS) and native Linux.
- The host needs **only a working Docker daemon**. Every client tool — `psql`,
  `curl`, `etcdctl`, the adversary — runs *inside* containers via `docker exec` /
  ephemeral `docker run`. No host `psql`/`curl`/`etcdctl`/`timeout`/`jq` required.
- The image is **self-contained** (multi-stage build compiles the workspace
  in-container), so there's no host Rust/toolchain dependency at run time.
- No MinIO/S3: media isn't part of the federation-trust surface, so nodes use
  `--media local`. Fewer moving parts, one less image.

## Components

- **image `democratos-byz`** (`Dockerfile`) — one image, three binaries: the real
  `democratos` node, `loadgen` (seeds an honest community), and `redteam` (the
  adversary).
- **`redteam`** (`crates/redteam`) — links the REAL `federation` crate, so every
  forgery is cryptographically valid: genuine Ed25519 signatures and genuine
  control-plane writes. It models an attacker with a node identity + the cluster
  token + etcd write access.
- **cluster** — a shared postgres (one DB per node) + single-node etcd, `NODES`
  democratos nodes (`1..HONEST` honest, the rest compromised), and one extra **rogue
  peer** (`redteam serve-rogue`) that serves a forged change-feed.

## The three "compromised node" surfaces tested

| Surface | What it is | Scenario |
|---|---|---|
| **External adversary** | holds cluster token + etcd access, node id 250 | G2, G5, X1, X2, X3 |
| **Captured cluster member** | a real compromised node (its own seed + published key) acting maliciously | G1, G3 |
| **Rogue / "forked" peer** | a node running attacker code (`serve-rogue`) that serves a forged feed | G4 |

## Run it

```bash
cd deploy/byzantine
./build.sh                       # once — compiles the workspace in-container (slow first time)

NODES=5 HONEST=2 ./up.sh         # Byzantine majority: 3 of 5 compromised
./byzantine.sh                   # run the scenarios
./down.sh

# larger / more extreme majority — the scripts are parameterized:
NODES=7 HONEST=2 ./up.sh && ./byzantine.sh && NODES=7 ./down.sh
```

`up.sh` seeds the honest community into node 1 **before node 1 boots**, so node 1
claims it and mints a founder-signed home binding at startup — giving a genuinely
founder-bound community to attack.

## Scenarios

Result classes: **GUARDRAIL** (`ok`/`bad`, must hold — a failure fails the suite),
**KNOWN-OPEN** (`xfail`, an audit finding the attack still wins), **PROBE**
(reported, never fails).

| # | Class | Attack (majority-compromised) → expected defence |
|---|-------|--------------------------------------------------|
| G1 | GUARDRAIL | compromised node tries to seize the bound community via etcd (rival community key + claim) → refused (first-write-wins + binding gate) |
| G2 | GUARDRAIL | external adversary signs a `demoi` rewrite, pushes to honest ingest → rejected `NotOwner` (0 applied, slug unchanged) |
| G3 | GUARDRAIL | a *captured* real cluster member forges the same event → rejected |
| G4 | GUARDRAIL | the rogue peer serves a forged feed; the honest node pulls it and rejects every event (slug stays `honest`, not `pwned`) |
| G5 | GUARDRAIL | repeated claims of the live-owned community never take ownership (epoch fencing / monotonicity) |
| G6 | GUARDRAIL (FED-1 closed) | durable takeover (community key + fencing-surviving binding) is blocked — the community-key publish is **origin-authenticated** (signed by the origin node key), so an attacker can't mint one |
| G7 | GUARDRAIL (HIGH-1 closed) | a forged comment is rejected, and comments now scope via their **parent post** (they replicate again — the liveness fix) |
| G8 | GUARDRAIL (FED-3 closed) | an attacker-signed home binding is **refused at `set_home_binding`** (verified against the community key), so it can neither be installed nor DoS the owner. Runs last |

> The former FED-1/FED-3 `xfail`s and the HIGH-1 probe are now guardrails (G6–G8),
> closed by the follow-up fixes. Residual by design: a *genuinely origin-less* community
> (no honest founding node ever published a key — e.g. the synthetic `d/999999`) has a
> bare lease that stays permissively claimable, but with no verifying key/binding there
> is no fencing-surviving seizure. Closing that too would break failover/import.

See `../../docs/security-audit-2026-07-10-verification.md` for the findings (FED-1,
FED-3, HIGH-1) these map to.

## Notes / limitations

- The rogue-peer guardrail (G4) reads the honest node's on-disk `demoi.slug`: had the
  forged feed been trusted, it would read `pwned`. The check is safe (it never
  false-passes into "secure"), but is only meaningful once node 1's puller has run a
  cycle against the rogue — `byzantine.sh` waits briefly first.
- X3 poisons the honest community's binding and is therefore **destructive**; it runs
  last, and you should `./down.sh && ./up.sh` before re-running guardrails.
- `redteam` prints a machine-readable `OUTCOME=<token>` line per attack; the shell
  decides pass/fail from the token (so "the attack succeeded" is not a process error).
