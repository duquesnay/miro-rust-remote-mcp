# Multi-stage build for Miro MCP Server
# Stage 1: Builder - compile Rust binary
FROM rust:1.90-bookworm AS builder

WORKDIR /build

# Copy dependency manifests first (for layer caching)
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary with optimizations
RUN cargo build --release --locked

# Verify binary was created
RUN ls -lh /build/target/release/miro-mcp-server

# Stage 2: Runtime - minimal Debian image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 -s /bin/bash mcp && \
    mkdir -p /app/data && \
    chown -R mcp:mcp /app

WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /build/target/release/miro-mcp-server /app/miro-mcp-server

# Set ownership
RUN chown mcp:mcp /app/miro-mcp-server

# Switch to non-root user
USER mcp

# Environment variables (override via Scaleway secrets)
ENV RUST_LOG=info
ENV TOKEN_STORAGE_PATH=/app/data/tokens.enc
ENV MCP_SERVER_PORT=3010

# Expose HTTP port for OAuth callbacks
EXPOSE 3010

# Volume for persistent token storage
VOLUME ["/app/data"]

# Health check using HTTP server endpoint
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3010/health || exit 1

# Run MCP server (stdio for MCP + HTTP for OAuth)
CMD ["/app/miro-mcp-server"]
