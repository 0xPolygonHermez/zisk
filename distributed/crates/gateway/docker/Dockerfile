# ── Build stage ───────────────────────────────────────────────────────────────
# Build context must be the workspace root so Cargo can resolve the full
# workspace. Build with:
#   docker build -f distributed/crates/gateway/docker/Dockerfile -t zisk-gateway .

FROM rust:1.85-slim AS builder

WORKDIR /workspace

# Install protobuf compiler (required by tonic build.rs)
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy the full workspace (needed for workspace Cargo resolution)
COPY . .

RUN cargo build --release -p zisk-gateway

# ── Runtime stage ─────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create unprivileged system user
RUN useradd --system --no-create-home --shell /usr/sbin/nologin zisk-gateway

# Create working directory
RUN mkdir -p /var/lib/zisk && chown zisk-gateway:zisk-gateway /var/lib/zisk

COPY --from=builder /workspace/target/release/zisk-gateway /usr/local/bin/zisk-gateway

# gRPC port | Prometheus metrics port
EXPOSE 7000 9090

USER zisk-gateway

ENTRYPOINT ["zisk-gateway", "--config", "/etc/zisk/gateway.toml"]
