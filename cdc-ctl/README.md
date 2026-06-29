# cdc-ctl

CLI for controlling the CDC daemon over gRPC and inspecting runtime configuration.

## Build

```bash
cargo check
```

## Commands

- `start` - launch daemon binary (foreground or background)
- `reload` - trigger daemon pipeline reload
- `stop` - request daemon shutdown
- `status` - query daemon health and metrics
- `print-config` - print masked runtime + parsed pipeline config

## Status Command

Displays daemon health, metrics, and optionally pipeline state.

```bash
# Basic status
cdc-ctl status

# Verbose mode (includes component health and active pipelines)
cdc-ctl status --verbose

# Custom daemon URL
cdc-ctl status --daemon-url http://remote-host:50051
```

Output includes:
- Overall health status (RUNNING, DEGRADED, etc.)
- Record counts (ingested, successfully sunk, failed, DLQ)
- Pipeline component states (verbose)
- Active pipeline list with cursor position and state (verbose)

## Environment Variables

`print-config` reads these values from environment (or `.env` when provided):

- `OTEL_EXPORTER_OTLP_ENDPOINT` (default: `http://localhost:4317`)
- `RW_HOST` (default: `localhost`)
- `RW_PORT` (default: `4566`)
- `RW_USER` (default: `root`)
- `RW_DBNAME` (default: `dev`)
- `OS_URL` (default: `https://localhost:9200`)
- `OS_USER` (default: `admin`)
- `OS_PASSWORD` (masked in output)
- `QDRANT_URL` (default: `https://localhost:6334`)
- `QDRANT_API_KEY` (masked in output)
- `PIPELINES_FILE` (default: `pipelines.yaml`)
- `HOSTNAME` (default: `local`)
- `CONSUMER_ID` (defaults to `HOSTNAME` when missing)

## Pipeline Config Schema

`print-config` supports YAML/TOML pipeline files compatible with the current daemon schema.

Required fields:

- `subscription_name`
- `sink_type` (`opensearch` or `qdrant`)
- `target_collection` (OpenSearch index or Qdrant collection)
- `id_field`
- `batch_size`

Optional fields:

- `vector_field` (used by `qdrant`; defaults to `embedding` in daemon)

Backward compatibility:

- `target_index` is accepted as an alias and mapped to `target_collection`.
- `sink_type` defaults to `opensearch` if omitted.

## Proto Source

gRPC bindings are generated from:

- `../cdc-daemon/proto/cdc_management.proto`
