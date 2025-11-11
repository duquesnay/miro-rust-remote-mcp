# Multi-stage build for Miro MCP Server (ADR-002 Resource Server)

# Stage 1: Build
FROM rust:1.83-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source to build dependencies (cached layer)
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "pub fn dummy() {}" > src/lib.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --bin server

# Remove dummy source
RUN rm -rf src

# Copy real source code
COPY src ./src

# Build with real source (fast - only compiles our code)
RUN touch src/lib.rs && \
    cargo build --release --bin server

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install CA certificates and minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/server /app/miro-mcp-server

# Expose port
EXPOSE 3010

# Set environment for production
ENV RUST_LOG=info
ENV PORT=3010

# Run the server
CMD ["/app/miro-mcp-server"]
