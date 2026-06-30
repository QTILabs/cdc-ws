# CDC Workspace Documentation Index

Use this page as the entry point for all project documentation.

## Canonical Docs (Primary)

1. [architecture-walkthrough.md](architecture-walkthrough.md)
- Architecture, control plane, security model, and enterprise operations baseline.

2. [daemon-configuration.md](daemon-configuration.md)
- Runtime configuration, environment variables, pipeline formats, and day-2 operations.

3. [image-generation-guide.md](image-generation-guide.md)
- Dockerfiles, image build and push workflows, and CI/CD image delivery guidance.

## Normative Specification

- [SPECS.md](SPECS.md)
- System contract for topology, telemetry, deployment baseline, and image versioning policy.

## Supporting Artifacts

- [k8s/cdc-starter.yaml](k8s/cdc-starter.yaml): starter Kubernetes deployment manifest.
- [examples/risingwave/postgres-to-opensearch.sql](examples/risingwave/postgres-to-opensearch.sql): runnable RisingWave CDC example DDL.

## Example Values

The repository still includes example-only placeholder values in a few starter files, including `.env` and `k8s/cdc-starter.yaml`.
These are intentional and must be replaced with environment-specific secrets, digests, and URLs before production use.

## Compatibility Docs

These files are intentionally kept as lightweight pointers for existing links:

- [development-guide.md](development-guide.md)
- [enterprise-implementation-guide.md](enterprise-implementation-guide.md)

## Suggested Reading Order

1. [README.md](README.md)
2. [architecture-walkthrough.md](architecture-walkthrough.md)
3. [daemon-configuration.md](daemon-configuration.md)
4. [image-generation-guide.md](image-generation-guide.md)
5. [SPECS.md](SPECS.md)


# cdc-multi-sink-ws

Workspace for CDC ingestion with multiple sink targets:

- `cdc-daemon`: core producer/consumer daemon (Postgres CDC -> OpenSearch/Qdrant)
- `cdc-ctl`: CLI control plane for daemon start/reload/stop/config inspection
- `cdc-bff`: REST + auth facade over daemon gRPC management APIs
- `cdc-web-console`: SolidStart web UI for health, metrics, auth, and pipeline operations

## Workspace Layout

- `cdc-daemon/`
- `cdc-ctl/`
- `cdc-bff/`
- `cdc-web-console/`
- `.env`

## Proto Contract

All gRPC clients are generated from:

- `cdc-daemon/proto/cdc_management.proto`

## Environment Variables

Set these in `.env` (workspace root) for local development.

### Daemon

- `RW_HOST` (default: `localhost`)
- `RW_PORT` (default: `4566`)
- `RW_USER` (default: `root`)
- `RW_DBNAME` (default: `dev`)
- `OS_URL` (default: `https://localhost:9200`)
- `OS_USER` (default: `admin`)
- `OS_PASSWORD` (required only when OpenSearch pipelines are active)
- `QDRANT_URL` (default: `https://localhost:6334`)
- `QDRANT_API_KEY` (optional)
- `PIPELINES_FILE` (default: `pipelines.yaml`)
- `OTEL_EXPORTER_OTLP_ENDPOINT` (default: `http://localhost:4317`)
- `CONSUMER_ID` (optional; defaults to `HOSTNAME`)
- `HOSTNAME` (default: `local`)
- `LOCAL_DLQ_DIR` (default in code if unset)

### BFF

- `CDC_DAEMON_GRPC_URL` (default: `http://localhost:50051`)
- `JWT_SECRET` (required)

Optional OAuth:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GITHUB_REDIRECT_URL`
- `KEYCLOAK_CLIENT_ID`
- `KEYCLOAK_CLIENT_SECRET`
- `KEYCLOAK_ISSUER`
- `KEYCLOAK_REDIRECT_URL`

## Pipeline Schema

Current daemon pipeline schema supports both sinks.

Required fields:

- `subscription_name`
- `sink_type` (`opensearch` or `qdrant`)
- `target_collection`
- `id_field`
- `batch_size`

Optional fields:

- `vector_field` (used by qdrant, defaults to `embedding`)

Compatibility note:

- `cdc-ctl` accepts `target_index` as an alias to `target_collection`.

## Quick Start

1. Build all components:

```bash
cd cdc-daemon && cargo check
cd ../cdc-ctl && cargo check
cd ../cdc-bff && cargo check
```

2. Start daemon:

```bash
cd ../cdc-ctl
cargo run -- start --foreground
```

3. Run BFF (separate terminal):

```bash
cd ../cdc-bff
cargo run
```

4. Reload pipelines after config edits:

```bash
cd ../cdc-ctl
cargo run -- reload
```

5. Start web console (separate terminal):

```bash
cd ../cdc-web-console
pnpm install
pnpm run dev
```

## Monitoring

### Daemon Status

Use `cdc-ctl status` to check daemon health and metrics:

```bash
# Basic status
cargo run --bin cdc-ctl -- status

# Detailed status (components + active pipelines)
cargo run --bin cdc-ctl -- status --verbose

# Remote daemon
cargo run --bin cdc-ctl -- status --daemon-url http://remote-host:50051
```

Output includes overall health, record counts (ingested/sunk/failed/DLQ), and optionally pipeline details.

The web console dashboard also shows per-sink counters (`sink_metrics`) for OpenSearch and Qdrant.

## Component Docs

- `cdc-daemon/` source and pipeline examples
- `cdc-ctl/README.md`
- `cdc-bff/README.md`


