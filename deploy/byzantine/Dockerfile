# Byzantine test-harness image. Unlike the root Dockerfile (a lean Pi runtime that
# ships only `democratos`), this bakes THREE binaries into one image so a single
# image serves every role in the harness:
#   * democratos — the real node (honest AND attacker-controlled nodes run this)
#   * loadgen    — seeds an honest community + voters straight into a node's DB
#   * redteam    — the adversary tool: forges control-plane writes and signed
#                  federation events using the REAL crypto (links the federation
#                  crate), to attack the cluster from a majority-compromised position
#
# Debug (not --release) on purpose: this image is rebuilt as the harness evolves and
# is never deployed, so fast compiles beat a lean binary. Platform-independent —
# builds the same on colima/arm64 (macOS) and native Linux; the host needs only
# Docker. Build from the repo root:  docker build -f deploy/byzantine/Dockerfile -t democratos-byz .

# ---- build stage ------------------------------------------------------------
FROM rust:1-bookworm AS builder
WORKDIR /src
# protobuf-compiler: etcd-client → tonic → prost-build needs `protoc` at build time
# (the control-plane adapter speaks gRPC to etcd).
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
COPY . .
# debuginfo=0: `redteam` statically links a large dep set (reqwest+rustls+sqlx+
# tonic+etcd+axum); linking it with full debug info OOM-kills `ld` in a small
# (e.g. 2 GB) builder VM. Dropping debug info slashes peak linker memory and image
# size. `-j 2` bounds parallel codegen memory. Build the biggest binary (redteam)
# in its own step so its link never overlaps the others'.
ENV CARGO_PROFILE_DEV_DEBUG=0
RUN cargo build --locked -j 2 -p democratos -p loadgen \
    && cargo build --locked -j 2 -p redteam \
    && cp target/debug/democratos target/debug/loadgen target/debug/redteam /usr/local/bin/

# ---- runtime stage ----------------------------------------------------------
FROM debian:bookworm-slim AS runtime
# curl: HEALTHCHECK + the harness drives HTTP probes via `docker exec <node> curl`,
# so no client tooling is needed on the host. ca-certificates: rustls trust roots.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --home-dir /home/app app \
    && mkdir -p /data/media \
    && chown -R app:app /data
USER app

COPY --from=builder /usr/local/bin/democratos /usr/local/bin/democratos
COPY --from=builder /usr/local/bin/loadgen    /usr/local/bin/loadgen
COPY --from=builder /usr/local/bin/redteam    /usr/local/bin/redteam

ENV DEMOCRATOS_RECOMMEND_INDEX=/tmp/democratos.recindex
EXPOSE 3000 7400
HEALTHCHECK --interval=5s --timeout=3s --start-period=10s --retries=5 \
    CMD curl -fsS http://127.0.0.1:3000/ >/dev/null || exit 1
# No ENTRYPOINT: the harness runs `democratos ...`, `loadgen ...`, or `redteam ...`
# explicitly per container.
