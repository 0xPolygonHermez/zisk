FROM rust:1.75-bullseye as builder

WORKDIR /app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the workspace
RUN cargo build --release --bin consensus-server

# Runtime stage
FROM debian:bullseye-slim

# Install necessary runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates tzdata curl && \
    rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false zisk

# Create log directory
RUN mkdir -p /var/log/consensus-network && chown zisk:zisk /var/log/consensus-network

# Copy the binary
COPY --from=builder /app/target/release/consensus-server /usr/local/bin/consensus-server

# Copy configuration
COPY config /etc/consensus-network/config

# Set permissions
RUN chown -R zisk:zisk /etc/consensus-network

# Switch to app user
USER zisk

# Set environment variables
ENV CONFIG_PATH=/etc/consensus-network/config/production
ENV CONSENSUS_LOGGING_FILE_OUTPUT=true
ENV CONSENSUS_LOGGING_FILE_PATH=/var/log/consensus-network/app.log

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the service
CMD ["consensus-server"]
