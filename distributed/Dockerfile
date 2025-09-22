FROM rust:1.75-bookworm as builder

WORKDIR /app

# Copy the entire workspace
COPY . .

# Build the distributed binaries
RUN cargo build --release --bin coordinator --bin prover

# Runtime stage
FROM debian:bookworm-slim

# Install necessary runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates tzdata && \
    rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -r -s /bin/false zisk

# Create log directory
RUN mkdir -p /var/log/distributed && chown zisk:zisk /var/log/distributed

# Copy the binaries
COPY --from=builder /app/target/release/coordinator /usr/local/bin/coordinator
COPY --from=builder /app/target/release/prover /usr/local/bin/prover

# Copy configuration files
COPY distributed/crates/coordinator/config /etc/distributed/coordinator/
COPY distributed/crates/prover/config /etc/distributed/prover/

# Set permissions
RUN chown -R zisk:zisk /etc/distributed

# Switch to app user
USER zisk

# Set environment variables
ENV CONFIG_PATH=/etc/distributed
ENV DISTRIBUTED_LOGGING_FILE_OUTPUT=true
ENV DISTRIBUTED_LOGGING_FILE_PATH=/var/log/distributed/app.log

# Expose port (coordinator gRPC port)
EXPOSE 50051

# Default to coordinator, but can be overridden
CMD ["coordinator"]
