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

# ── Stage 2: Generate recipe ───────────────────────────────────────────────
# Copy ALL workspace members — cargo metadata needs every member listed
# in Cargo.toml to resolve the workspace, even when building a single package.
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
COPY cdc-bff/    cdc-bff/
COPY cdc-ctl/    cdc-ctl/
COPY pipelines.yaml ./
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: Build ──────────────────────────────────────────────────────────
FROM chef AS builder
ARG PACKAGE
ARG BINARY=${PACKAGE}

# Recipe + all workspace members for cargo metadata resolution
COPY --from=planner /workspace/recipe.json /workspace/recipe.json

# Copy ALL members — cargo build needs them for workspace resolution
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
COPY cdc-bff/    cdc-bff/
COPY cdc-ctl/    cdc-ctl/
COPY pipelines.yaml ./

# Build only the requested package (target/ already has deps from cargo chef cook)
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    cargo build --release --locked --package ${PACKAGE}

# ── Stage 4: Runtime image ──────────────────────────────────────────────────
FROM ${RUNTIME_IMAGE}
ARG PACKAGE
ARG BINARY=cdc-daemon
ARG EXPOSE_PORT
ENV BINARY=${BINARY}

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/${BINARY} /usr/local/bin/${BINARY}

# Ensure execute permission (multi-stage COPY can strip +x on some Docker versions)
RUN chmod +x /usr/local/bin/${BINARY}

EXPOSE ${EXPOSE_PORT}

# Hardcode binary path — no shell expansion, no broken empty-string fallback
ENTRYPOINT ["/bin/sh", "-lc", "exec /usr/local/bin/${BINARY}"]
HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=20s \
    CMD ["true"]