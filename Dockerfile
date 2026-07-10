# Democratos — container image, tuned for small ARM boxes (Raspberry Pi).
#
# Multi-stage: a fat Rust toolchain builds a lean static-ish binary, then we copy
# just that binary into a slim runtime. Build for the Pi's architecture with
# buildx, e.g.:
#
#   docker buildx build --platform linux/arm64 -t democratos:latest --load .
#
# (Pi 4/5 on a 64-bit OS is arm64. Use linux/arm/v7 for 32-bit Pi OS.)
# Cross-building on a faster machine is far quicker than compiling on the Pi.

# ---- build stage ------------------------------------------------------------
FROM rust:1-bookworm AS builder
WORKDIR /src

# Copy the whole workspace and build the release binary. (For faster iterative
# builds, cargo-chef can cache the dependency layer; omitted here to keep the
# image definition simple and obviously correct.)
COPY . .
RUN cargo build --release --locked -p democratos \
    && cp target/release/democratos /usr/local/bin/democratos

# ---- runtime stage ----------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# curl is only here for the container HEALTHCHECK; everything else is the binary.
RUN apt-get update \
    && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Run as an unprivileged user; /data is where durable state lives (mount a volume).
RUN useradd --system --create-home --home-dir /home/app app \
    && mkdir -p /data/media \
    && chown -R app:app /data
USER app

COPY --from=builder /usr/local/bin/democratos /usr/local/bin/democratos

# Sensible in-container defaults; override any via `environment:` in compose.
ENV DEMOCRATOS_STORE=file \
    DEMOCRATOS_DATA=/data/democratos.json \
    DEMOCRATOS_MEDIA_DIR=/data/media \
    DEMOCRATOS_RECOMMEND_INDEX=/tmp/democratos.recindex \
    DEMOCRATOS_ADDR=0.0.0.0:3000

EXPOSE 3000

# The server must bind 0.0.0.0 (set above) to be reachable from outside the
# container.
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:3000/ >/dev/null || exit 1

ENTRYPOINT ["democratos"]
CMD ["serve"]
