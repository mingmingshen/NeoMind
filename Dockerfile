# =============================================================================
# NeoMind Dockerfile — Multi-stage build
# =============================================================================
# Usage:
#   docker build -t neomind:latest .
#   docker compose up -d
#
# Platforms: linux/amd64, linux/arm64
# =============================================================================

# ---------------------------------------------------------------------------
# Stage 1: Build frontend
# ---------------------------------------------------------------------------
FROM --platform=$BUILDPLATFORM node:20-alpine AS frontend

WORKDIR /build/web

# Install dependencies first (layer cache)
COPY web/package.json web/package-lock.json ./
RUN npm ci --ignore-scripts

# Copy source and build
COPY web/ ./
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 2: Build backend
# ---------------------------------------------------------------------------
FROM --platform=$TARGETPLATFORM rust:1.85-alpine AS backend

# jemalloc (used by neomind-cli as the global allocator) must assume 64KB
# pages on ARM, else it crashes on 64KB-page hosts like Raspberry Pi 5 and
# Jetson (same fix as the bare-metal ARM release — see release-build-glibc22.04).
# No-op on amd64 (4KB pages, jemalloc default).
ARG TARGETARCH

RUN apk add --no-cache musl-dev

WORKDIR /build

# Cache dependencies by creating a dummy build first
COPY Cargo.toml Cargo.lock ./
COPY crates/neomind-core/Cargo.toml crates/neomind-core/Cargo.toml
COPY crates/neomind-api/Cargo.toml crates/neomind-api/Cargo.toml
COPY crates/neomind-agent/Cargo.toml crates/neomind-agent/Cargo.toml
COPY crates/neomind-cli/Cargo.toml crates/neomind-cli/Cargo.toml
COPY crates/neomind-cli-ops/Cargo.toml crates/neomind-cli-ops/Cargo.toml
COPY crates/neomind-storage/Cargo.toml crates/neomind-storage/Cargo.toml
COPY crates/neomind-devices/Cargo.toml crates/neomind-devices/Cargo.toml
COPY crates/neomind-rules/Cargo.toml crates/neomind-rules/Cargo.toml
COPY crates/neomind-messages/Cargo.toml crates/neomind-messages/Cargo.toml
COPY crates/neomind-extension-sdk/Cargo.toml crates/neomind-extension-sdk/Cargo.toml
COPY crates/neomind-extension-runner/Cargo.toml crates/neomind-extension-runner/Cargo.toml
COPY crates/neomind-data-push/Cargo.toml crates/neomind-data-push/Cargo.toml

# Create dummy source files for dependency caching
RUN mkdir -p crates/neomind-core/src && echo "" > crates/neomind-core/src/lib.rs && \
    mkdir -p crates/neomind-api/src && echo "fn main(){}" > crates/neomind-api/src/lib.rs && \
    mkdir -p crates/neomind-agent/src && echo "" > crates/neomind-agent/src/lib.rs && \
    mkdir -p crates/neomind-cli/src && echo "fn main(){}" > crates/neomind-cli/src/main.rs && \
    mkdir -p crates/neomind-cli-ops/src && echo "" > crates/neomind-cli-ops/src/lib.rs && \
    mkdir -p crates/neomind-storage/src && echo "" > crates/neomind-storage/src/lib.rs && \
    mkdir -p crates/neomind-devices/src && echo "" > crates/neomind-devices/src/lib.rs && \
    mkdir -p crates/neomind-rules/src && echo "" > crates/neomind-rules/src/lib.rs && \
    mkdir -p crates/neomind-messages/src && echo "" > crates/neomind-messages/src/lib.rs && \
    mkdir -p crates/neomind-extension-sdk/src && echo "" > crates/neomind-extension-sdk/src/lib.rs && \
    mkdir -p crates/neomind-extension-runner/src && echo "" > crates/neomind-extension-runner/src/lib.rs && \
    mkdir -p crates/neomind-data-push/src && echo "" > crates/neomind-data-push/src/lib.rs

RUN if [ "$TARGETARCH" = "arm64" ] || [ "$TARGETARCH" = "aarch64" ]; then export JEMALLOC_SYS_WITH_LG_PAGE=16; fi && \
    cargo build --release -p neomind-cli -p neomind-extension-runner 2>/dev/null || true

# Copy real source code and build
COPY crates/ crates/
RUN if [ "$TARGETARCH" = "arm64" ] || [ "$TARGETARCH" = "aarch64" ]; then export JEMALLOC_SYS_WITH_LG_PAGE=16; fi && \
    cargo build --release -p neomind-cli -p neomind-extension-runner

# ---------------------------------------------------------------------------
# Stage 3: Runtime
# ---------------------------------------------------------------------------
FROM alpine:3.22 AS runtime

# `apk upgrade` patches base-image packages, keeping the shipped image clear
# of known alpine CVEs between base-image refreshes (the main source of
# "high" findings in image scans). Then add the runtime deps.
RUN apk upgrade --no-cache && apk add --no-cache ca-certificates curl tzdata && \
    addgroup -S neomind && adduser -S neomind -G neomind

WORKDIR /app

# Copy backend binaries (neomind finds extension-runner in same directory or PATH)
COPY --from=backend /build/target/release/neomind /usr/local/bin/neomind
COPY --from=backend /build/target/release/neomind-extension-runner /usr/local/bin/neomind-extension-runner

# Copy frontend build output
COPY --from=frontend /build/web/dist /var/www/neomind

# Create data directory
RUN mkdir -p /app/data && chown -R neomind:neomind /app/data

# Environment defaults
ENV NEOMIND_WEB_DIR=/var/www/neomind
ENV RUST_LOG=neomind=info
ENV RUST_BACKTRACE=1

EXPOSE 9375 1883

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:9375/api/health || exit 1

USER neomind

VOLUME ["/app/data"]

ENTRYPOINT ["neomind"]
CMD ["serve"]
