# NeoMind Backend - Multi-stage Docker build for multi-platform

# Stage 1: Build
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy source code
COPY . .

# Build the API server in release mode
# This will build for the default target architecture
RUN cargo build --release -p neomind-api

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 neomind

# Set working directory
WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/neomind-api /app/neomind-api

# Create data directory
RUN mkdir -p /data && chown -R neomind:neomind /data

# Switch to non-root user
USER neomind

# Expose default port
EXPOSE 9375

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget -q --spider http://localhost:9375/api/health || exit 1

# Set environment variables
ENV RUST_LOG=info
ENV NEOMIND_DATA_DIR=/data
ENV NEOMIND_BIND_ADDR=0.0.0.0:9375

# Run the server
CMD ["/app/neomind-api"]
