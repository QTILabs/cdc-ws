# CDC Daemon Configuration

This document explains how to configure the CDC daemon and the BFF gateway that sits in front of it.

Important: the daemon itself does not handle OpenID login. OpenID/OAuth settings belong to the BFF gateway in `cdc-bff/src/main.rs` and `cdc-bff/src/auth.rs`.

## 1. What each component uses

- CDC daemon: reads from the source database subscription, writes to the sink, and reports metrics over gRPC.
- BFF gateway: handles local login and OAuth/OpenID sign-in, then calls the daemon over gRPC.

## 2. CDC daemon settings

The daemon reads configuration from environment variables and a pipelines file.

### Environment file (.env)

Use the workspace `.env` file for daemon settings.

- File location: `./.env`
- The daemon loads this file on startup via `dotenvy::dotenv()`.
- Values in `.env` are treated as environment variables at runtime.
- If a value is missing in `.env`, the daemon falls back to built-in defaults where available.
- `OS_PASSWORD` is required and must be present.

### Required settings

- `OS_PASSWORD`: OpenSearch password. This is required.
- `PIPELINES_FILE`: Path to the pipeline definition file. Defaults to `pipelines.yaml`.

### Source settings

These settings define the source connection used by the daemon to read from the upstream database or subscription system.

- `RW_HOST`: Source host. Defaults to `localhost`.
- `RW_PORT`: Source port. Defaults to `4566`.
- `RW_USER`: Source username. Defaults to `root`.
- `RW_DBNAME`: Source database name. Defaults to `dev`.
- `CONSUMER_ID`: Optional consumer identifier. Defaults to the machine hostname.
- `HOSTNAME`: Optional hostname label. Defaults to `local` when missing.

The daemon builds the source connection string as:

```text
host=<RW_HOST> port=<RW_PORT> user=<RW_USER> dbname=<RW_DBNAME> sslmode=require
```

### Sink settings

These settings define where the daemon writes the captured rows.

- `OS_URL`: OpenSearch endpoint. Defaults to `https://localhost:9200`.
- `OS_USER`: OpenSearch username. Defaults to `admin`.
- `OS_PASSWORD`: OpenSearch password. Required.

### Telemetry settings

- `OTEL_EXPORTER_OTLP_ENDPOINT`: OpenTelemetry collector endpoint. Defaults to `http://localhost:4317`.

### DLQ settings

- `LOCAL_DLQ_DIR`: Local dead-letter-queue directory override.
- Default DLQ directory: `/var/log/cdc-dlq`.

## 3. Pipeline file format

`PIPELINES_FILE` points to a YAML or TOML file containing pipeline definitions.

### YAML example

```yaml
- subscription_name: public.customers_sub
  target_index: customers
  id_field: id
  batch_size: 500
- subscription_name: public.orders_sub
  target_index: orders
  id_field: order_id
  batch_size: 500
```

### TOML example

```toml
[[pipelines]]
subscription_name = "public.customers_sub"
target_index = "customers"
id_field = "id"
batch_size = 500

[[pipelines]]
subscription_name = "public.orders_sub"
target_index = "orders"
id_field = "order_id"
batch_size = 500
```

Field meanings:

- `subscription_name`: Source subscription or cursor the daemon should read from.
- `target_index`: OpenSearch index to write into.
- `id_field`: JSON field used as the document ID.
- `batch_size`: Number of rows fetched per source read.

## 4. BFF OpenID / OAuth settings

These settings are configured in the BFF, not in the daemon.

### JWT settings

- `JWT_SECRET`: Secret used to sign JWTs.
- Default: `super_secret_key_change_me`.

### OAuth/OpenID settings currently wired in the code

The BFF currently enables the `github` provider when these values are present:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GITHUB_REDIRECT_URL`

Default redirect URL:

```text
http://localhost:8080/api/auth/oauth2/github/callback
```

The login flow uses PKCE and requests the `openid` and `profile` scopes.

Note: the current BFF code only wires GitHub automatically. If you want a second OpenID Connect provider such as `oidc`, you need to add it in `cdc-bff/src/main.rs` using the same provider pattern.

### Keycloak OIDC guide

The daemon does not authenticate users directly. To use Keycloak for daemon access, configure Keycloak in the BFF and keep the daemon behind the BFF.

1. Create a Keycloak client
- In Keycloak Admin, create a client for the web console and BFF flow.
- Protocol: OpenID Connect.
- Client type: confidential.
- Enable Standard Flow.
- Add redirect URI: `http://localhost:8080/api/auth/oauth2/keycloak/callback`.

2. Collect Keycloak endpoints
- Realm issuer: `http://<keycloak-host>/realms/<realm-name>`.
- Authorization endpoint: `<issuer>/protocol/openid-connect/auth`.
- Token endpoint: `<issuer>/protocol/openid-connect/token`.
- UserInfo endpoint: `<issuer>/protocol/openid-connect/userinfo`.

3. Add BFF environment variables

```bash
export KEYCLOAK_CLIENT_ID="cdc-bff"
export KEYCLOAK_CLIENT_SECRET="<client-secret>"
export KEYCLOAK_ISSUER="http://localhost:8081/realms/cdc"
export KEYCLOAK_REDIRECT_URL="http://localhost:8080/api/auth/oauth2/keycloak/callback"
```

4. Wire the provider in BFF startup
- Add a `keycloak` provider entry in `cdc-bff/src/main.rs` in the same map where `github` is registered.
- Set:
  - `auth_url = ${KEYCLOAK_ISSUER}/protocol/openid-connect/auth`
  - `token_url = ${KEYCLOAK_ISSUER}/protocol/openid-connect/token`
  - `userinfo_url = ${KEYCLOAK_ISSUER}/protocol/openid-connect/userinfo`
  - `redirect_url = ${KEYCLOAK_REDIRECT_URL}`
  - `client_id = ${KEYCLOAK_CLIENT_ID}`
  - `client_secret = ${KEYCLOAK_CLIENT_SECRET}`

5. Validate end-to-end access
- Start Keycloak, daemon, and BFF.
- Log in via `/api/auth/oauth2/keycloak/login`.
- Verify protected routes such as `/api/cdc/health` return success with BFF-issued JWT.

## 5. Example environment setup

```bash
# Source
export RW_HOST="localhost"
export RW_PORT="4566"
export RW_USER="root"
export RW_DBNAME="dev"

# Sink
export OS_URL="https://localhost:9200"
export OS_USER="admin"
export OS_PASSWORD="change-me"

# Pipelines
export PIPELINES_FILE="pipelines.yaml"

# Telemetry
export OTEL_EXPORTER_OTLP_ENDPOINT="http://localhost:4317"

# Optional labels
export CONSUMER_ID="cdc-worker-1"
export HOSTNAME="cdc-worker-1"
export LOCAL_DLQ_DIR="/var/log/cdc-dlq"

# BFF auth
export JWT_SECRET="change-me-too"
export GITHUB_CLIENT_ID="..."
export GITHUB_CLIENT_SECRET="..."
export GITHUB_REDIRECT_URL="http://localhost:8080/api/auth/oauth2/github/callback"

# Optional: Keycloak OIDC
export KEYCLOAK_CLIENT_ID="cdc-bff"
export KEYCLOAK_CLIENT_SECRET="..."
export KEYCLOAK_ISSUER="http://localhost:8081/realms/cdc"
export KEYCLOAK_REDIRECT_URL="http://localhost:8080/api/auth/oauth2/keycloak/callback"
```

## 6. Runtime ports

- BFF REST API: `8080`
- CDC daemon gRPC API: `50051`
- OpenTelemetry collector: `4317`
- OpenSearch default endpoint: `9200`

## 7. cdc-ctl operational commands

Use `cdc-ctl` from the workspace root for daemon lifecycle and configuration operations.

### Start daemon

```bash
# Start daemon in background (default)
cargo run -p cdc-ctl -- start

# Start daemon in foreground
cargo run -p cdc-ctl -- start --foreground

# Start daemon using an explicit binary path
cargo run -p cdc-ctl -- start --daemon-bin ./target/debug/cdc-daemon
```

### Hot reload pipeline configuration

```bash
cargo run -p cdc-ctl -- reload --daemon-url http://localhost:50051
```

### Stop daemon gracefully

```bash
cargo run -p cdc-ctl -- stop --daemon-url http://localhost:50051
```

### Print loaded configuration with obfuscation

```bash
# Uses .env and PIPELINES_FILE by default
cargo run -p cdc-ctl -- print-config

# Explicit files
cargo run -p cdc-ctl -- print-config --env-file .env --pipelines-file pipelines.yaml
```

`print-config` masks values for fields containing `password`, `secret`, `token`, or key-like names before printing.

## 8. Operational notes

- The daemon currently uses `sslmode=require` for the source database connection.
- The daemon reads `.env` at startup.
- The daemon reads the pipelines file at startup. Use the web console "Reload daemon" action after writing a new pipeline configuration.
- The BFF talks to the daemon over gRPC using `CDC_DAEMON_GRPC_URL`, which defaults to `http://localhost:50051`.
- The BFF front-end routes should point to `/api/auth/...` and `/api/cdc/...` through the gateway.
