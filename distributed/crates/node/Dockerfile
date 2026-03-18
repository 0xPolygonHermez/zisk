# syntax=docker/dockerfile:1
#
# Build context: workspace root
#   docker build -f distributed/crates/node/Dockerfile -t zisklet .
#   docker run -v ./config:/etc/zisk:ro -p 7000:7000 zisklet

ARG HEALTH_PROBE_VERSION=v0.4.34

# ── Stage 1: cargo-chef planner ───────────────────────────────────────────────
FROM rust:1.93-bookworm AS chef
RUN cargo install cargo-chef --locked
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 2: dependency cache ─────────────────────────────────────────────────
# This layer is rebuilt only when Cargo.toml / Cargo.lock change.
FROM chef AS builder
ARG HEALTH_PROBE_VERSION
ARG TARGETARCH=amd64

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Download grpc_health_probe for the target architecture.
RUN curl -fsSL -o /usr/local/bin/grpc_health_probe \
    "https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${HEALTH_PROBE_VERSION}/grpc_health_probe-linux-${TARGETARCH}" \
    && chmod +x /usr/local/bin/grpc_health_probe

COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# ── Stage 3: build ────────────────────────────────────────────────────────────
COPY . .
RUN cargo build --release -p zisk-distributed-node

# ── Stage 4: runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --no-create-home --shell /usr/sbin/nologin zisklet \
    && mkdir -p /etc/zisk /var/lib/zisk \
    && chown zisklet:zisklet /var/lib/zisk

COPY --from=builder /build/target/release/zisklet /usr/local/bin/zisklet
COPY --from=builder /usr/local/bin/grpc_health_probe /usr/local/bin/grpc_health_probe

USER zisklet
EXPOSE 7000

ENTRYPOINT ["zisklet"]
CMD ["--config", "/etc/zisk/node.toml"]
