# Deploying Democratos as a federation

Democratos scales horizontally by **sharding communities across nodes**, not by
sharing one big database. Each node runs its own Postgres and is the *source of
truth* for the communities it owns; every other node keeps a synced read-only
replica. Reads are local; writes forward to the owner; a dead node's communities
rehome onto a quieter one. This directory deploys that cluster.

- **Single box** (a Raspberry Pi, one JSON file): use the repo-root
  [`docker-compose.yml`](../docker-compose.yml) instead. Nothing here is needed.
- **Federation** (many nodes, shared media + control plane): use
  [`docker-compose.federation.yml`](docker-compose.federation.yml), below.

## What's in the stack

| Service        | Role                                                                    |
| -------------- | ----------------------------------------------------------------------- |
| `node1`,`node2`| App nodes. Each owns communities, serves reads from its replica.        |
| `db1`, `db2`   | Per-node Postgres — each node's authoritative store.                    |
| `etcd`         | Control plane: ownership leases + epoch fencing + load reporting.       |
| `minio`        | Shared S3 bucket for media, so **any node serves any upload**.          |
| `caddy`        | Public reverse proxy; load-balances reads across the nodes.             |

Two networks keep the trust boundary explicit:

- **`edge`** — only the reverse proxy ↔ app web ports (`:3000`).
- **`fabric`** — databases, the federation feed/command ports (`:7400`), etcd,
  and media. **Never published.** The federation ports carry authenticated,
  signed traffic but are still kept off the public internet as defence in depth.

The only published port is the proxy's `:8080`.

## Run it

```sh
docker compose -f deploy/docker-compose.federation.yml up -d --build
docker compose -f deploy/docker-compose.federation.yml logs -f node1 node2
# open http://localhost:8080
```

Scale to more nodes by copying a `nodeN` service: give it a unique
`DEMOCRATOS_NODE_ID`, its own `dbN` + `DATABASE_URL`, a fresh
`DEMOCRATOS_NODE_SEED`, and a `--peer <id>=http://<other>:7400` for each peer it
should replicate from. Add it to the `reverse_proxy` upstream list in
[`Caddyfile`](Caddyfile).

## Seeding content: the private dev node (only you can switch accounts)

To populate the cluster with content — switching between a fixed set of fake
accounts and posting as each — use
[`docker-compose.dev-federation.yml`](docker-compose.dev-federation.yml). It is
the same federation plus one extra **private dev node** whose account switcher
only *you* can reach:

```sh
cp deploy/.env.example deploy/.env      # then fill in FRESH secrets (see below)
docker compose -f deploy/docker-compose.dev-federation.yml up -d --build
# public site (anyone):   http://localhost:8080
# your switcher (only you): http://localhost:8090/dev/unlock?key=$DEMOCRATOS_DEV_UNLOCK_SECRET
```

Then reload `http://localhost:8090` and use the floating dev bar to switch
between the puppet accounts and post as each.

Four layers keep the switcher yours alone — no one on the public site (or your
network) can switch accounts:

1. **`node1`/`node2` have the switcher OFF.** Every `/dev/*` endpoint `404`s on
   the public `:8080` site.
2. **The dev node is network-isolated.** `node-dev` is not on the public `edge`
   network, so the public proxy can never route to it. It is fronted only by
   `caddy-dev`, published on **`127.0.0.1:8090`** — reachable from your machine
   and nowhere else.
3. **Unlock needs a secret.** `GET /dev/unlock` hands out the switcher cookie only
   with `?key=$DEMOCRATOS_DEV_UNLOCK_SECRET`; a missing/wrong key is an
   indistinguishable `404`. The app also **refuses to boot** a `--dev` node on a
   non-loopback bind without that secret, so it is fail-closed by construction.
4. **Puppets can never vote.** `DEMOCRATOS_DEV_ACCOUNTS` are created permanently
   *franchise-barred*: the switcher acts only as these accounts, and the domain
   guarantees a barred account can never become a voter (not via enfranchisement,
   founding, or co-signing). Set real governance up with normal sign-ups.

Generate `DEMOCRATOS_DEV_UNLOCK_SECRET` like any other secret
(`head -c32 /dev/urandom | xxd -p -c32`) and keep it off the public site.

### On a Raspberry Pi (arm64) with a big media disk

The images are multi-arch, so the same compose runs on a Pi. Build natively on
the Pi, or cross-build the app image:

```sh
docker buildx build --platform linux/arm64 -t democratos:latest --load .
```

Media is the only store that grows without bound. Point it at the Pi's large
drive by setting **`MEDIA_DIR`** in `deploy/.env` to an absolute path on that
disk (it is bind-mounted into MinIO; the dir must exist and be writable):

```sh
MEDIA_DIR=/mnt/storage/democratos-media
```

Left unset, media uses a managed Docker volume. The per-node Postgres volumes stay
small; only `MEDIA_DIR` needs the roomy disk.

## Security — do this before any real deployment

The committed seeds, tokens, and passwords are **placeholders**. Replace them:

```sh
# a per-node signing identity (one per node, keep them secret & stable)
head -c32 /dev/urandom | xxd -p -c32     # -> DEMOCRATOS_NODE_SEED
# the shared node-to-node bearer token
head -c24 /dev/urandom | xxd -p -c24     # -> DEMOCRATOS_CLUSTER_TOKEN
```

Move them into a `.env` file or Docker/Compose secrets rather than inlining them.
Key facts about the security model:

- **`DEMOCRATOS_NODE_SEED`** is the node's Ed25519 identity. It signs every event
  the node replicates; peers verify the signature *and* that the signer is the
  community's rightful owner at a non-stale epoch. Losing/rotating it makes a
  node's past events unverifiable — treat it like a TLS private key.
- **`DEMOCRATOS_CLUSTER_TOKEN`** gates the feed/command/ingest HTTP endpoints. A
  request without it gets `401`. It is a coarse network gate; the per-event
  signatures are what actually prevent forged votes.
- **TLS on the database link.** On a single Docker host the `fabric` network is
  local, so the plaintext `postgres://…@db1` link doesn't leave the box — the
  node still prints a startup warning (by design). If you split nodes across
  hosts, terminate the DB link over TLS and use `?sslmode=verify-full` in
  `DATABASE_URL`; the warning then goes away. Likewise front the proxy with a
  real domain so Caddy provisions HTTPS (see the [`Caddyfile`](Caddyfile) note).
- **Vote durability.** Votes are acked only after replicating to a standby
  (quorum of 2) and fail closed otherwise. For zero loss even if an owner
  crashes mid-replication, additionally run a synchronous Postgres physical
  replica for each `dbN` — orthogonal to this app-level replication.

## Backfilling from a single-box deployment

Migrate an existing `democratos.json` into a node's Postgres, IDs preserved:

```sh
# copy the snapshot in, then run the importer against that node's DB
docker compose -f deploy/docker-compose.federation.yml cp \
    ./democratos.json node1:/data/democratos.json
docker compose -f deploy/docker-compose.federation.yml exec node1 \
    democratos --store postgres import --from /data/democratos.json
```

The import is **idempotent** (re-running inserts only what's missing) and
advances the node's ID counters past the imported IDs, so newly created entities
never collide. The node then claims those communities and its peers replicate
them. Media referenced by imported posts must also be uploaded to the shared
bucket (the app re-uploads on write; pre-existing local files can be copied into
MinIO under the same content-addressed keys).

## Watching failover / rehoming

```sh
# take a node down; after ~one lease TTL its communities rehome onto the peer
docker compose -f deploy/docker-compose.federation.yml stop node1
docker compose -f deploy/docker-compose.federation.yml logs -f node2   # "rehomed community …"
docker compose -f deploy/docker-compose.federation.yml start node1     # returns as a replica; old epoch is fenced
```

## Production hardening checklist

- Run etcd as a **3-node** cluster (this stack uses one for simplicity).
- Give each app node a **standby** peer for every community it owns, so
  sync-replicated votes always have a quorum.
- Set `fabric` to `internal: true` once images are pulled/registry-mirrored, so
  the data network has no egress.
- Put real resource limits (`mem_limit`, `cpus`) on each service for your host.
- Back up each `dbN` volume and the MinIO bucket; the etcd state is
  reconstructable (nodes re-claim on boot) but backing it up speeds recovery.
