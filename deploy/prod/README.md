# Deploying `demo.ratbum.com` — two-host federation

A hardened, two-box federation behind one public HTTPS site.

| Host            | Hardware        | Role                                                                                          |
| --------------- | --------------- | --------------------------------------------------------------------------------------------- |
| **192.168.2.5** | Raspberry Pi 5  | Full / public node (**node 1**): its own Postgres + the **shared** etcd & MinIO + public Caddy |
| **192.168.2.4** | Raspberry Pi 4  | Content node (**node 2**): its own Postgres + a TLS front for its federation feed              |

Both Pis run Ubuntu Server (arm64) and are **too slow to compile Rust**, so the images are
**cross-built on your Mac** (`linux/arm64`) and shipped as a tarball — the Pis only *run*
containers, never build them. (If the Pi ↔ IP mapping is reversed, swap the IPs below.)

```
                 internet
                    │  (demo.ratbum.com A → 63.135.77.235)
             router 80/443 forward
                    ▼
   ┌─────────────── .5 ───────────────┐        ┌──────── .4 ────────┐
   │ caddy:443 ──▶ node1:3000 (web)    │        │ caddy:443 ──▶ node2 │
   │        └───▶ node1:7400 (feed n1) │        │        (feed n2)    │
   │ etcd:2379 (mTLS)   minio (via caddy)│      │ db2                 │
   │ db1                                │◀──────│ node2 ⇄ federation  │
   └────────────────────────────────────┘  LAN  └─────────────────────┘
```

**Trust model on the wire**

- Public site: Let's Encrypt cert on `demo.ratbum.com` (router-forwarded 80/443 → .5).
- Peer feed + MinIO between hosts: publicly-trusted Let's Encrypt certs via **Cloudflare
  DNS-01** (the node's `reqwest`/`rust-s3` clients trust only public CA roots, so a
  self-signed cert would be rejected — this is why we don't just use an internal CA here).
- etcd control plane: its **own internal CA + mutual TLS** (the app natively trusts a
  custom CA for etcd via `DEMOCRATOS_ETCD_CA`), published on `.5:2379` for `.4` to reach.
- Every replicated event is Ed25519-signed; per-account signatures are mandatory under
  federation. Accounts are honoured fleet-wide only from nodes holding an issuer cert that
  verifies against the offline `FEDERATION_TRUST_ROOT`.

---

## Prerequisites you (only you) must do

These are outside what the deploy scripts can touch:

1. **DNS (Cloudflare, ratbum.com).** Create these records. The fabric names point at
   **private** LAN IPs, so they must be **DNS-only (grey cloud), NOT proxied**:

   | Name               | Type | Value            | Proxy         |
   | ------------------ | ---- | ---------------- | ------------- |
   | `demo.ratbum.com`  | A    | `63.135.77.235`  | DNS-only\*    |
   | `n1.ratbum.com`    | A    | `192.168.2.5`    | **DNS-only**  |
   | `n2.ratbum.com`    | A    | `192.168.2.4`    | **DNS-only**  |
   | `minio.ratbum.com` | A    | `192.168.2.5`    | **DNS-only**  |
   | `etcd.ratbum.com`  | A    | `192.168.2.5`    | **DNS-only**  |

   \*`demo` can be proxied (orange) later if you want Cloudflare in front; keep it DNS-only
   for the first bring-up so Caddy's cert and your testing are unambiguous.

2. **Cloudflare API token.** Create a token with **Zone → DNS → Edit** on the `ratbum.com`
   zone. Put it in both `.env` files as `CLOUDFLARE_API_TOKEN` (Caddy uses it for DNS-01).

3. **Router port-forward.** Forward public **TCP 80 and 443 → 192.168.2.5** only. Do **not**
   forward anything to .4, and never forward 2379/7400/9000.

4. **SMTP.** You said the relay is already set up on the box — record its host/port/user/pass
   and a `From:` address in the `.env` files (`DEMOCRATOS_SMTP_*`). If it listens on the host
   itself, see the note in `host5/.env.example`.

5. **Ubuntu prep on each Pi** (.5 and .4) — Docker + the deploy files, but **no compiler**:
   ```sh
   sudo apt-get update && sudo apt-get install -y ca-certificates curl git
   curl -fsSL https://get.docker.com | sudo sh          # Docker Engine + compose plugin (arm64)
   sudo usermod -aG docker $USER && newgrp docker       # run docker without sudo
   git clone <this-repo> ~/democratos                   # only for the deploy/prod files + certs;
                                                         # the app itself arrives as a prebuilt image
   ```
   The runtime base images (postgres, minio, etcd, the app, caddy) are all multi-arch and pull
   their arm64 variants automatically.

---

## One-time federation ceremony (run on your workstation, offline)

The root seed is the master key for account trust — it never touches a node.

```sh
cd deploy/prod

# 1. Generate the trust root. Prints FEDERATION_ROOT_SEED (KEEP OFFLINE) and
#    FEDERATION_TRUST_ROOT (goes in BOTH .env files).
cargo run -q -p democratos -- issuer root
#   → save FEDERATION_ROOT_SEED to a password manager / offline file
#   → copy FEDERATION_TRUST_ROOT into host5/.env and host4/.env

# 2. Sign an issuer cert for each node (offline; uses the root seed).
export FEDERATION_ROOT_SEED=<the secret seed from step 1>
cargo run -q -p democratos -- issuer certify --node 1 --epoch 1 > node1.issuer.json
cargo run -q -p democratos -- issuer certify --node 2 --epoch 1 > node2.issuer.json
unset FEDERATION_ROOT_SEED
#   (These .json certs are safe to copy to the hosts; they're published in step 6 below.)

# 3. Generate the etcd internal CA + server cert + per-node client certs.
./gen-etcd-certs.sh          # writes deploy/prod/etcd-certs/
```

**Generate the shared secrets** (same values must match where noted in the `.env` files):

```sh
head -c32 /dev/urandom | xxd -p -c32   # DEMOCRATOS_CLUSTER_TOKEN  (same on both)
openssl rand -hex 32                    # DEMOCRATOS_SESSION_SECRET (same on both)
head -c32 /dev/urandom | xxd -p -c32   # DEMOCRATOS_ADMIN_SECRET   (same on both)
head -c32 /dev/urandom | xxd -p -c32   # NODE1_SEED   (host5 only)
head -c32 /dev/urandom | xxd -p -c32   # NODE2_SEED   (host4 only, distinct)
head -c24 /dev/urandom | xxd -p -c24   # MINIO_ROOT_PASSWORD, DB_PASSWORD, etc.
```

---

## Build the arm64 images on your Mac and ship them

```sh
cd deploy/prod
./build-images.sh                 # -> deploy/prod/democratos-pi-images.tar.gz (linux/arm64)

# copy to both Pis
scp democratos-pi-images.tar.gz <you>@192.168.2.5:~/
scp democratos-pi-images.tar.gz <you>@192.168.2.4:~/
```

On **each** Pi, load the images once:
```sh
gunzip -c ~/democratos-pi-images.tar.gz | docker load   # loads democratos:latest + democratos-caddy:latest
```

Because both images are now present locally, `docker compose up -d` (below, **no `--build`**)
uses them directly and never tries to compile on the Pi.

---

## Testing before you go public (optional but recommended)

You don't need Cloudflare, DNS, TLS, or the router forward to validate that the build runs and
the federation works. Three tiers, lightest first. All reuse the `democratos:latest` image you
loaded above, so nothing compiles. **Run these on a Pi** — a stock Docker install (from
`get.docker.com`) has the `compose` plugin; a Mac using Colima + the Homebrew Docker CLI does
not, so `docker compose` won't run there.

**Tier 1 — smoke-test the image (no DNS/TLS/router/federation):**
```sh
cd ~/democratos
docker compose up -d                     # repo-root single-box compose; app only, on :3000
curl -I http://192.168.2.5:3000          # expect 200
docker compose down
```
Proves the arm64 image boots and serves. (Binds a non-loopback address with no session secret,
so it logs an ephemeral-key warning — expected for a throwaway test.)

**Tier 2 — the whole federation on ONE box (no public/TLS):**
```sh
cd ~/democratos
cp deploy/.env.example deploy/.env && $EDITOR deploy/.env   # fill fresh secrets
docker compose -f deploy/docker-compose.federation.yml up -d
# open http://192.168.2.5:8080   (node1 + node2 + etcd + minio + caddy, all internal/plaintext)
docker compose -f deploy/docker-compose.federation.yml logs -f node1 node2
```
Exercises replication, media→MinIO, and rehoming over the isolated internal network — **no
Cloudflare, DNS, or router needed**. This is the best behavioural test before going public.
(Uses the plaintext-federation escape hatch, fine for a local test box, not for production.)

**Tier 3 — the real prod stack, LAN-only first:**
Do the Cloudflare DNS records + API token (needed for cert issuance) and the ceremony below, and
bring up `host5`/`host4` — but **defer the router port-forward**. Test over the LAN before
exposing anything:
```sh
curl -k --resolve demo.ratbum.com:443:192.168.2.5 https://demo.ratbum.com   # served by node1 on .5
```
When that's green, add the 80/443 → .5 forward on the router to go public. This shakes out the
production config (etcd mTLS, cross-host feeds, real Let's Encrypt certs) with nothing yet
reachable from the internet.

---

## Host 192.168.2.5 (node 1) — bring-up

```sh
cd ~/democratos/deploy/prod/host5
cp .env.example .env && $EDITOR .env          # fill EVERY value

# etcd + node1 certs from the ceremony (copy from your workstation):
mkdir -p certs
#   from deploy/prod/etcd-certs/ :
#     ca.pem                -> certs/etcd-ca.pem
#     etcd-server.pem       -> certs/etcd-server.pem
#     etcd-server-key.pem   -> certs/etcd-server-key.pem
#     node1-client.pem      -> certs/etcd-client.pem
#     node1-client-key.pem  -> certs/etcd-client-key.pem

docker compose up -d            # uses the loaded images; NO --build (never compiles on the Pi)
docker compose logs -f node1 caddy
```

Watch for: `node1` becomes healthy; Caddy logs "certificate obtained" for `demo`, `n1`,
`minio`; no "refusing plaintext" or "session secret" boot errors.

## Host 192.168.2.4 (node 2) — bring-up

```sh
cd ~/democratos/deploy/prod/host4
cp .env.example .env && $EDITOR .env          # MinIO/cluster/trust-root/session/admin MUST match .5

mkdir -p certs
#   from deploy/prod/etcd-certs/ :
#     ca.pem                -> certs/etcd-ca.pem
#     node2-client.pem      -> certs/etcd-client.pem
#     node2-client-key.pem  -> certs/etcd-client-key.pem

docker compose up -d            # uses the loaded images; NO --build
docker compose logs -f node2 caddy
```

## Publish the issuer certs (once both nodes are up)

Run on **.5** (it has etcd access). This tells the control plane that node 1 and node 2 are
trusted account issuers, so their accounts are honoured across the federation.

```sh
cd ~/democratos/deploy/prod/host5
for n in 1 2; do
  docker compose exec -T \
    -e FEDERATION_TRUST_ROOT="$(grep -E '^FEDERATION_TRUST_ROOT=' .env | cut -d= -f2-)" \
    node1 democratos issuer publish \
      --cert-file /certs/node${n}.issuer.json \
      --etcd-endpoints https://etcd.ratbum.com:2379
done
```
(Copy `node1.issuer.json` / `node2.issuer.json` into `host5/certs/` first so the container
can read them.)

---

## Verify

```sh
# public site, real cert:
curl -I https://demo.ratbum.com

# federation is live: node1 replicating node2 and vice-versa (no auth/TLS errors)
docker compose -f host5/docker-compose.yml logs node1 | grep -Ei 'replicat|rehomed|peer|issuer'
docker compose -f host4/docker-compose.yml logs node2 | grep -Ei 'replicat|peer|issuer'

# invite-only admin queue reachable only from the LAN + with the secret:
curl -s -o /dev/null -w '%{http_code}\n' \
  "https://demo.ratbum.com/admin?key=<DEMOCRATOS_ADMIN_SECRET>"   # 200 from a .5/.4 host; 404 elsewhere
```

Then create the first real (non-puppet) account through the invite flow, found a community,
and confirm media upload works (it lands in the shared MinIO bucket, served via `/media/...`).

---

## Security notes / knobs

- **Nothing runs on a committed credential** — every secret is `${VAR:?}` in compose and the
  app refuses to boot with placeholder session secrets on a public bind.
- **Only .5 is internet-exposed** (80/443). etcd (2379), the feeds (7400), and MinIO (9000)
  are LAN-only and additionally authenticated (mTLS / cluster token / Ed25519). Firewall the
  LAN so only .4 and .5 reach each other's fabric ports if the LAN is shared.
- **CSAM scanning is OFF** (`DEMOCRATOS_CSAM_SCAN` unset) — no lawful hash source is wired.
  Malicious-media re-encoding still runs. See `docs/media-safety.md` to enable it with a
  real corpus.
- **Rotating an issuer**: `issuer certify --node N --epoch <higher>` then `issuer publish`.
- **Rehoming**: stop a node; after one lease TTL its communities rehome onto the peer; on
  restart its old epoch is fenced. See the top-level `deploy/README.md`.
