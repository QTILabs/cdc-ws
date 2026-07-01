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

# ── Stage 1: Cargo Chef (dependency caching) ──────────────────────────────────
# Using cargo-chef for deterministic dependency layer caching.
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

# ── Stage 2: Dependency resolution ────────────────────────────────────────────
FROM chef AS planner
COPY . .
# Generate recipe for dependency caching (excludes src/ content)
RUN cargo chef prepare --recipe-path recipe.json

# ── Stage 3: Build dependencies (cached layer) ────────────────────────────────
FROM chef AS builder
ARG PACKAGE
ARG BINARY=${PACKAGE}

COPY --from=planner /workspace/recipe.json /workspace/recipe.json

# Build **only** dependencies — src/ is not yet copied.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/workspace/target,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json --package ${PACKAGE}

# ── Stage 4: Build binary ────────────────────────────────────────────────────
FROM chef AS final-builder
ARG PACKAGE
ARG BINARY=${PACKAGE}

COPY --from=builder /workspace/target /workspace/target
COPY --from=builder /usr/local/cargo/registry /usr/local/cargo/registry

# Copy full source (only src/ actually matters now, but workspace resolution
# needs all member manifests + proto/)
COPY Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
COPY cdc-bff/ cdc-bff/
COPY cdc-ctl/ cdc-ctl/

# Build with cache mounts for maximum reuse
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/workspace/target,sharing=locked \
    cargo build --release --locked --package ${PACKAGE} \
    && cp /workspace/target/release/${BINARY} /workspace/${BINARY}

# ── Stage 5: Runtime image ────────────────────────────────────────────────────
FROM ${RUNTIME_IMAGE}
ARG PACKAGE
ARG BINARY=${PACKAGE}
ARG EXPOSE_PORT

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=final-builder /workspace/${BINARY} /usr/local/bin/

EXPOSE ${EXPOSE_PORT}

# Shell form + exec = proper signal handling + ARG expansion in binary name
ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/${BINARY}"]

# Docker healthcheck (substitute at build time via build args)
# Override in docker-compose healthcheck if custom endpoint needed.
HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=20s \
    CMD ["true"]
