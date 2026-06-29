# cdc-bff

REST BFF for CDC daemon management APIs with local/OAuth authentication.

## Build

```bash
cargo check
```

## Runtime

Default bind address:

- `0.0.0.0:8080`

Daemon gRPC connection:

- `CDC_DAEMON_GRPC_URL` (default: `http://localhost:50051`)

## Authentication Environment Variables

Required for local JWT auth:

- `JWT_SECRET`

Optional OAuth providers:

GitHub:

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`
- `GITHUB_REDIRECT_URL` (default: `http://localhost:8080/api/auth/oauth2/github/callback`)

Keycloak:

- `KEYCLOAK_CLIENT_ID`
- `KEYCLOAK_CLIENT_SECRET`
- `KEYCLOAK_ISSUER`
- `KEYCLOAK_REDIRECT_URL` (default: `http://localhost:8080/api/auth/oauth2/keycloak/callback`)

## API Endpoints

Auth:

- `POST /api/auth/login`
- `GET /api/auth/oauth2/{provider}/login`
- `GET /api/auth/oauth2/{provider}/callback`

CDC (JWT required):

- `GET /api/cdc/health`
- `GET /api/cdc/metrics`
- `GET /api/cdc/pipelines`
- `POST /api/cdc/pipelines/reload`

## Pipeline Field Note

`GET /api/cdc/pipelines` returns daemon `PipelineStatus` values and currently exposes `target_index` in the response body for compatibility with the gRPC contract.

## Proto Source

gRPC bindings are generated from:

- `../cdc-daemon/proto/cdc_management.proto`
