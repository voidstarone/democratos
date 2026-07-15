# Federation test & stress harness

A one-machine rig that stands up a **real** multi-node Democratos federation and
exercises it for correctness and load. It runs each node as a host process
against the same containers the Rust test-suite already uses (one Postgres, one
etcd, one MinIO) — each node gets its **own database** in that Postgres, so the
topology (per-node source-of-truth + shared control-plane + shared media) is the
real one, just packed onto a laptop.

Everything here also applies unchanged to the production
[`docker-compose.federation.yml`](../docker-compose.federation.yml) cluster —
point `loadgen`'s `--nodes` / `--owner-db` at it instead.

## Prerequisites

Build the binaries and start the three backing containers:

```sh
cargo build -p democratos -p loadgen

docker run -d --name democratos-pg-test   -p 55432:5432 \
  -e POSTGRES_USER=app -e POSTGRES_PASSWORD=pg -e POSTGRES_DB=democratos postgres:16-alpine
docker run -d --name democratos-etcd-test -p 52379:2379 \
  gcr.io/etcd-development/etcd:v3.5.16 /usr/local/bin/etcd \
  --listen-client-urls http://0.0.0.0:2379 --advertise-client-urls http://127.0.0.1:2379
docker run -d --name democratos-minio-test -p 59000:9000 \
  -e MINIO_ROOT_USER=minioadmin -e MINIO_ROOT_PASSWORD=minioadmin minio/minio server /data
```

## Running the Rust DB-gated tests

Most of the Rust suite runs storeless, but the Postgres adapter and federation
integration tests are gated on `TEST_DATABASE_URL` and **silently skip** when it
is unset. Point it at a scratch database on the `democratos-pg-test` container
above (the tests apply all migrations on connect and create sibling databases as
needed, so `app` needs the `CREATEDB` it has by default):

```sh
docker exec democratos-pg-test psql -U app -d postgres -c 'CREATE DATABASE democratos_test'
export TEST_DATABASE_URL='postgres://app:pg@127.0.0.1:55432/democratos_test'
cargo test -p adapter-store-postgres -p adapter-federation
```

> Migration versions must be **unique** across `crates/adapter-store-postgres/migrations/`
> — sqlx keys `_sqlx_migrations` on the numeric prefix, so two files sharing an
> `NNNN` (easy to do when branches land in parallel) make *every* migrate run
> fail with a `_sqlx_migrations_pkey` duplicate. Renumber, don't collide.

## The scripts

| Script          | What it does                                                                 |
| --------------- | --------------------------------------------------------------------------- |
| `./up.sh`       | Create per-node DBs and launch `NODES` nodes (peered, one etcd, one bucket). |
| `./functional.sh` | Assert the design guarantees: replication, forwarding, no double-vote, sync durability, convergence. |
| `./stress.sh`   | Seed a large electorate, run a concurrent vote storm + read storm, verify.   |
| `./chaos.sh`    | Kill an owner; assert failover, fencing (no split-brain), integrity, no stall. |
| `./down.sh`     | Stop the nodes, drop the DBs, wipe the etcd control-plane state.             |

Typical sessions:

```sh
NODES=2 ./up.sh && ./functional.sh ; ./down.sh                       # correctness
NODES=3 ./up.sh && VOTERS=2000 CONCURRENCY=128 ./stress.sh ; ./down.sh   # load
NODES=3 ./up.sh && ./chaos.sh ; ./down.sh                            # failover
```

Tunables (env): `NODES`, `VOTERS`, `CONCURRENCY`, `READS`, `DB_POOL`. Because all
nodes share one test Postgres (`max_connections≈100`), `DB_POOL` defaults to
`80/NODES`; a real deployment gives each node its own Postgres and sizes pools
far higher.

## `loadgen`

The driver (`crates/loadgen`) speaks the app's real HTTP surface. The app
identifies the acting user by a `uid` cookie, so it "logs in" as any voter by
sending `uid=<id>`. Subcommands:

- `seed`   — create a community + open proposal + N eligible voters straight into
  the owner's Postgres (through the real store, so IDs and the outbox are correct
  and it replicates). Writes a manifest.
- `vote`   — every voter casts once, spread across the node web URLs, at a chosen
  concurrency; reports latency percentiles, throughput, and an error breakdown.
  Votes cast on a non-owner node forward to the owner, so this drives the whole
  federated write path.
- `verify` — assert the authoritative tally, that no voter is double-counted, and
  that a replica DB converges (with the lag).
- `read`   — GET throughput across nodes.

## What "working as intended" looks like

On a clean cluster you should see (representative laptop numbers):

- **functional**: 5/5 — replication mirrors the owner; a vote on a non-owner node
  lands on the owner; the same voter is refused a second ballot on another node;
  an owner-accepted vote is on the standby immediately; the replica converges.
- **stress** (1000 voters, 3 nodes, concurrency 64): ~300 votes/s, **0 rejected /
  0 errored**, authoritative tally exactly = voters, replica converges in a few
  seconds; reads ~4–5k req/s at p50 ≈ 12 ms.
- **chaos**: the community rehomes off the killed owner within ~1 lease TTL; the
  returned old owner is **fenced** (does not reclaim — no split-brain); ballots
  stay one-per-voter; and replication does **not** stall on the old owner's now-
  fenced events.

## Findings — bugs this harness surfaced (and fixed)

Building and running this rig found six real defects, all now fixed with
regression tests (`crates/adapter-federation/tests/{ingest,gateway}.rs`,
`crates/adapter-store-postgres/tests/*`):

1. **Lost writes in the found/boot window.** A peer that pulled a brand-new
   community's events before its owner claimed it (a few-second gap) dropped them
   permanently. Fix: the feed is a strict ordered log — a *transient* rejection
   (`Unowned`) stops the cursor so the events are retried once ownership settles.
2. **Cursor corruption from the sync-vote push.** The synchronous push shared the
   async puller's cursor and could advance it past events the puller hadn't
   delivered, orphaning them. Fix: the push is a cursor-free idempotent pre-apply
   (`apply_rows`); the puller alone owns the cursor.
3. **Duplicate-key crash under concurrent votes.** Concurrent pushes carried
   overlapping rows and raced on `DELETE`+`INSERT`. Fix: the push path uses
   `INSERT … ON CONFLICT DO NOTHING`.
4. **Replication stall after failover.** A returned old owner's feed contains
   events now fenced (`NotOwner`/`StaleEpoch`); the strict log stalled on them,
   blocking all later replication. Fix: *permanent* rejections are skipped and the
   cursor steps past them (transient vs permanent classification).
5. **Votes failed on a fresh cluster.** Standbys were only designated during a
   failover, so a just-booted owner had none and every sync vote failed closed.
   Fix: an owner designates a standby for each community it claims.
6. **Sync-vote throughput collapse.** Draining the outbox stamped each event's
   epoch with a separate control-plane read — O(events) per push. Fix: cache the
   epoch per community per drain. ~9× faster (32 → ~300 votes/s).

## Known limitations (by design or deferred)

- **Un-acked votes are not durable across a failover.** A vote is only
  acknowledged after reaching a standby (quorum of 2, fail-closed). A vote that
  committed on an owner but whose standby push failed returns an error to the user
  *and* may be dropped if that owner then dies. This is the deliberate
  integrity-over-availability contract; for zero loss also run a Postgres
  synchronous physical replica per node.
- **Two live nodes can't form a fresh quorum after one dies.** With `NODES=2`,
  killing a node leaves the survivor unable to meet quorum-of-2, so it correctly
  fails votes closed. Use ≥3 nodes for continued vote availability under one
  failure.
- **Standby re-designation after rehoming can lag.** The new owner may briefly
  lack a live standby (so votes fail closed) until a later heartbeat picks one —
  a self-healing availability dip, not a correctness issue.
- **etcd lease stability.** The lease TTL is 15 s; a chronically overloaded etcd
  can flap ownership, which (correctly) pauses replication of the affected
  community until it settles. Size/replicate etcd for production.
