# Multi-stage Dockerfile with GPU support via ARG
# Uses stable image versions and follows best practices

# Build stage
FROM rust:bookworm AS builder

# Build arguments
ARG GPU=false

# Install build dependencies efficiently
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential pkg-config libssl-dev bash sudo curl gnupg ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy and install system dependencies (including GPU if needed)
COPY tools/test-env/ ./tools/test-env/
RUN chmod +x ./tools/test-env/*.sh && \
    cd tools/test-env && \
    if [ "$GPU" = "true" ]; then \
    echo "Installing with GPU support..." && \
    ./install_deps.sh --gpu; \
    else \
    echo "Installing without GPU support..." && \
    ./install_deps.sh; \
    fi

# Copy source code and build binaries (.dockerignore will exclude unnecessary files)
COPY . .

RUN ulimit -n 65535 && \
    cargo build --release \
    --package zisk-distributed-coordinator \
    --package zisk-distributed-worker \
    --package zisk-witness

# Runtime stage - minimal image
FROM debian:bookworm-slim

# Build arguments for runtime stage
ARG GPU=false

# Install build tools needed for dependency script
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 bash sudo curl gnupg && \
    rm -rf /var/lib/apt/lists/*

# Copy dependency installation scripts to runtime
COPY tools/test-env/ ./tools/test-env/
RUN chmod +x ./tools/test-env/*.sh

# Install runtime dependencies (including GPU if needed)
RUN cd tools/test-env && \
    if [ "$GPU" = "true" ]; then \
    echo "Installing runtime dependencies with GPU support..." && \
    ./install_deps.sh --gpu; \
    else \
    echo "Installing runtime dependencies without GPU support..." && \
    ./install_deps.sh; \
    fi

# Clean up installation scripts after use
RUN rm -rf ./tools/test-env

# Create non-root user for security
RUN groupadd -r zisk && useradd -r -g zisk -d /app -s /bin/false zisk

WORKDIR /app

# Create directory structure with proper ownership
RUN mkdir -p bin config/coordinator config/worker /app/proofs /var/log/distributed /app/.zisk/cache /app/.zisk/bin && \
    chown -R zisk:zisk /var/log/distributed /app/.zisk /app/bin /app/config /app/proofs

# Copy binaries from builder stage
COPY --from=builder --chown=zisk:zisk /app/target/release/zisk-coordinator ./bin/
COPY --from=builder --chown=zisk:zisk /app/target/release/zisk-worker ./bin/
# Copy the witness library
COPY --from=builder --chown=zisk:zisk /app/target/release/libzisk_witness.so ./bin/

# Copy configuration files with proper ownership
COPY --chown=zisk:zisk distributed/crates/coordinator/config/ ./config/coordinator/
COPY --chown=zisk:zisk distributed/crates/worker/config/ ./config/worker/

# Create compatibility copies in .zisk/bin after binaries are in place  
RUN cp /app/bin/* /app/.zisk/bin/

# Set up environment and permissions
ENV PATH="/app/bin:$PATH" \
    RUST_LOG="info"

RUN chmod +x bin/*

# Health check for monitoring
HEALTHCHECK --interval=30s --timeout=10s --start-period=15s --retries=3 \
    CMD pgrep -f "zisk-" > /dev/null || exit 1

# Switch to non-root user for security
USER zisk

# Expose the default port
EXPOSE 50051

# Default command (can be overridden in docker-compose)
CMD ["zisk-coordinator", "--config", "/app/config/coordinator/prod.toml"]
