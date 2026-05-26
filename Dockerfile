# Multi-stage build for all GZ Ads services.
# Build arg BIN selects which binary to produce.
#
# Usage:
#   docker build --build-arg BIN=bidder       -t gz-bidder .
#   docker build --build-arg BIN=campaign-api -t gz-api .
#   docker build --build-arg BIN=pacing       -t gz-pacing .
#   docker build --build-arg BIN=event-consumer -t gz-events .

ARG BIN=bidder

# ── Builder ────────────────────────────────────────────────────────────────────
FROM rust:1.82-slim-bookworm AS builder

ARG BIN

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/           ./crates/
COPY migrations/       ./migrations/

RUN cargo build --release --bin ${BIN}

# ── Runtime ────────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

ARG BIN
ENV RUST_LOG=info

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/${BIN} /usr/local/bin/app

CMD ["/usr/local/bin/app"]
