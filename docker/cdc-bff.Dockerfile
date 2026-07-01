# syntax=docker/dockerfile:1.7

# ── Stage 1: Build Rust binary ──────────────────────────────────
FROM rust:1.88-slim-bookworm AS builder
WORKDIR /workspace

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        libssl-dev \
        pkg-config \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY cargo.toml Cargo.lock ./
COPY proto ./proto

# Copy all workspace members (needed for workspace resolution)
COPY cdc-daemon/cargo.toml cdc-daemon/cargo.toml
COPY cdc-daemon/build.rs  cdc-daemon/build.rs
COPY cdc-daemon/src/      cdc-daemon/src/

COPY cdc-bff/cargo.toml cdc-bff/cargo.toml
COPY cdc-bff/build.rs  cdc-bff/build.rs
COPY cdc-bff/src/      cdc-bff/src/

COPY cdc-ctl/cargo.toml cdc-ctl/cargo.toml
COPY cdc-ctl/build.rs  cdc-ctl/build.rs
COPY cdc-ctl/src/      cdc-ctl/src/

RUN cargo build --locked --release -p cdc-bff

# ── Stage 2: Runtime image ──────────────────────────────────────
FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/cdc-bff /usr/local/bin/cdc-bff

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/cdc-bff"]
