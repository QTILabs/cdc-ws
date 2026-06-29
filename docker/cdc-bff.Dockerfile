# syntax=docker/dockerfile:1.7

FROM rust:1.88-bookworm AS builder
WORKDIR /workspace

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        libssl-dev \
        pkg-config \
        protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY proto ./proto
COPY cdc-bff/cargo.toml cdc-bff/Cargo.toml
COPY cdc-bff/build.rs cdc-bff/build.rs
COPY cdc-bff/src ./cdc-bff/src
COPY cdc-daemon/cargo.toml cdc-daemon/Cargo.toml
COPY cdc-daemon/build.rs cdc-daemon/build.rs
COPY cdc-daemon/src ./cdc-daemon/src
COPY cdc-ctl/cargo.toml cdc-ctl/Cargo.toml
COPY cdc-ctl/build.rs cdc-ctl/build.rs
COPY cdc-ctl/src ./cdc-ctl/src

RUN cargo build --locked --release -p cdc-bff

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /workspace/target/release/cdc-bff /usr/local/bin/cdc-bff

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/cdc-bff"]
