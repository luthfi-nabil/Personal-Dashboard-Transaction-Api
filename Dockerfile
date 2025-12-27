# ---------- Build stage ----------
FROM rust:1.75-bookworm AS builder

# Create app directory
WORKDIR /app

# Copy dependency files first (better layer caching)
COPY Cargo.toml Cargo.lock ./

# Create empty src to prebuild dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy actual source code
COPY . .

# Build real binary
RUN cargo build --release

# ---------- Runtime stage ----------
FROM debian:bookworm-slim

# Install runtime dependencies (TLS, certificates)
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/app /app/app