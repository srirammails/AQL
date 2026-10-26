# Build stage
FROM rust:latest AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    cmake \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy source code
COPY . .

# Build release binary
RUN cargo build --release --package clawdb-server

# Runtime stage - use trixie to match builder's glibc
FROM debian:trixie-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/clawdb /usr/local/bin/clawdb

# Create data directory
RUN mkdir -p /data

# Expose HTTP/MCP port and gRPC port
EXPOSE 3000 50051

# Default command - run HTTP + gRPC server with persistent data
CMD ["clawdb", "--path", "/data", "serve", "--port", "3000", "--grpc-port", "50051"]
