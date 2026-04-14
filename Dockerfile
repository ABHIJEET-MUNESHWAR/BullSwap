# =============================================================================
# BullSwap Dockerfile — Multi-stage build for minimal production image
# =============================================================================
#
# Build:  docker build -t bullswap .
# Run:    docker run -p 8080:8080 --env-file .env bullswap
#
# Final image is ~25 MB (scratch + static binary + migrations + CA certs)
# =============================================================================

# ---------------------------------------------------------------------------
# Stage 1: Build the release binary
# ---------------------------------------------------------------------------
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Install system dependencies needed by sqlx (OpenSSL headers not needed — we use rustls)
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cache dependency compilation: copy only manifests first
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs so `cargo build` compiles deps without real source
RUN mkdir src && \
    echo 'fn main() { println!("dummy"); }' > src/main.rs && \
    echo 'pub fn dummy() {}' > src/lib.rs

# Build dependencies only (cached unless Cargo.toml/lock change)
# Set SQLX_OFFLINE to skip compile-time DB checks during Docker build
ENV SQLX_OFFLINE=true
RUN cargo build --release && rm -rf src

# Copy real source code + migrations
COPY src/ src/
COPY migrations/ migrations/
COPY benches/ benches/

# Touch main.rs to invalidate the dummy build
RUN touch src/main.rs src/lib.rs

# Build the real binary
RUN cargo build --release --bin bullswap

# ---------------------------------------------------------------------------
# Stage 2: Minimal runtime image
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies (ca-certificates for TLS, curl for health check)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user for security
RUN groupadd -r bullswap && useradd -r -g bullswap -d /app -s /sbin/nologin bullswap

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/bullswap /app/bullswap

# Copy migrations (needed at runtime for sqlx::migrate!)
COPY --from=builder /app/migrations /app/migrations

# Set ownership
RUN chown -R bullswap:bullswap /app

# Switch to non-root user
USER bullswap

# Expose the default port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the server
# HOST=0.0.0.0 is required to accept connections from outside the container
ENV HOST=0.0.0.0
ENV PORT=8080
ENV LOG_LEVEL=info
ENV BATCH_INTERVAL_SECS=30
ENV MAX_ORDERS_PER_BATCH=1000

ENTRYPOINT ["/app/bullswap"]

