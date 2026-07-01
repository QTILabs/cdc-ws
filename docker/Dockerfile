# syntax=docker/dockerfile:1.7

# =============================================================================
# CDC Workspace — Unified Rust Dockerfile
# =============================================================================
# Build using:
#   docker build \
#     -f docker/Dockerfile \
#     --build-arg PACKAGE=cdc-daemon \
#     --build-arg BINARY=cdc-daemon \
#     --build-arg EXPOSE_PORT=50051 \
#     -t cdc-daemon:latest .
#
# Or via docker-compose (see docker-compose.yml)
# =============================================================================

ARG RUST_IMAGE=rust:1.88-slim-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

# ── Stage 1: Cargo Chef install ─────────────────────────────────────────────
# cargo-chef generates a recipe.json for deterministic dependency caching.
FROM ${RUST_IMAGE} AS chef
WORKDIR /workspace

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        libssl-dev \
        pkg-config \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-chef --locked --version 0.1.70

# ── Stage 2: Generate recipe ────────────────────────────────────────────────
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: Build ──────────────────────────────────────────────────────────
FROM chef AS builder
ARG PACKAGE
ARG BINARY=${PACKAGE}

# Recipe only (no src/) — dependency-only build for caching
COPY --from=planner /workspace/recipe.json /workspace/recipe.json

# Cook dependencies (target written to real filesystem, NOT cache mount)
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json --package ${PACKAGE}

# Copy full workspace source (all member manifests required for resolution)
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
COPY cdc-bff/   cdc-bff/
COPY cdc-ctl/   cdc-ctl/

# Build final binary (target/ already has deps from cargo chef cook)
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release --locked --package ${PACKAGE} \
    && cp target/release/${BINARY} /workspace/${BINARY}

# ── Stage 4: Runtime image ──────────────────────────────────────────────────
FROM ${RUNTIME_IMAGE}
ARG BINARY
ARG EXPOSE_PORT

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/${BINARY} /usr/local/bin/

EXPOSE ${EXPOSE_PORT}

# Shell form + exec = proper signal handling + ARG expansion in binary name
ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/${BINARY}"]

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=20s \
    CMD ["true"]
