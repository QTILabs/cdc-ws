# Image Generation Guide

This guide standardizes container image generation for:

- cdc-daemon
- cdc-bff
- cdc-ctl
- cdc-web-console

## 1. Prerequisites

Install and verify:

- Docker 24+ (or compatible OCI builder)
- Docker Buildx
- Registry access (for example GHCR, ECR, GCR, ACR)

Optional but recommended:

- Enable BuildKit (`DOCKER_BUILDKIT=1`)

## 2. Recommended file layout

The canonical Dockerfiles live in the `docker/` directory:

- `docker/cdc-daemon.Dockerfile`
- `docker/cdc-bff.Dockerfile`
- `docker/cdc-ctl.Dockerfile`
- `docker/cdc-web-console.Dockerfile`

This keeps Docker context at the repository root so the Rust workspace, lockfile, and proto files are available during build.
The checked-in files are the source of truth; the sections below summarize the same build strategy without duplicating the full file contents.

## 3. Rust service image templates

### 3.1 cdc-daemon image

See [docker/cdc-daemon.Dockerfile](docker/cdc-daemon.Dockerfile) for the checked-in multi-stage build. It compiles the daemon in a Rust builder image and copies only the release binary plus CA certificates into the runtime image.

### 3.2 cdc-bff image

See [docker/cdc-bff.Dockerfile](docker/cdc-bff.Dockerfile) for the checked-in multi-stage build. It follows the same pattern as the daemon image and exposes port 8080 for the BFF HTTP service.

### 3.3 cdc-ctl image

See [docker/cdc-ctl.Dockerfile](docker/cdc-ctl.Dockerfile) for the checked-in multi-stage build. It produces a slim operational CLI image suitable for one-shot jobs and admin tasks.

## 4. Web console image template

See [docker/cdc-web-console.Dockerfile](docker/cdc-web-console.Dockerfile) for the checked-in frontend build. It builds the static Vite output with pnpm and serves it from nginx.

## 5. Build commands

Set your registry namespace and version tag:

```bash
export REGISTRY=ghcr.io/your-org
export VERSION=0.1.0
```

Build images from workspace root:

```bash
docker build -f docker/cdc-daemon.Dockerfile -t ${REGISTRY}/cdc-daemon:${VERSION} .
docker build -f docker/cdc-bff.Dockerfile -t ${REGISTRY}/cdc-bff:${VERSION} .
docker build -f docker/cdc-ctl.Dockerfile -t ${REGISTRY}/cdc-ctl:${VERSION} .
docker build -f docker/cdc-web-console.Dockerfile -t ${REGISTRY}/cdc-web-console:${VERSION} .
```

## 6. Multi-architecture build and push

Use Buildx for linux amd64 and arm64:

```bash
docker buildx create --use --name cdc-builder || true

docker buildx build --platform linux/amd64,linux/arm64 \
  -f docker/cdc-daemon.Dockerfile \
  -t ${REGISTRY}/cdc-daemon:${VERSION} \
  --push .

docker buildx build --platform linux/amd64,linux/arm64 \
  -f docker/cdc-bff.Dockerfile \
  -t ${REGISTRY}/cdc-bff:${VERSION} \
  --push .

docker buildx build --platform linux/amd64,linux/arm64 \
  -f docker/cdc-ctl.Dockerfile \
  -t ${REGISTRY}/cdc-ctl:${VERSION} \
  --push .

docker buildx build --platform linux/amd64,linux/arm64 \
  -f docker/cdc-web-console.Dockerfile \
  -t ${REGISTRY}/cdc-web-console:${VERSION} \
  --push .
```

## 7. Runtime environment notes

- cdc-daemon needs source, sink, pipelines, and OTEL environment variables.
- cdc-bff needs `CDC_DAEMON_GRPC_URL`, `JWT_SECRET`, and OIDC env values.
- cdc-web-console should be fronted by ingress and point API traffic to cdc-bff.
- cdc-ctl image is typically used for operational jobs or one-shot administration.

## 8. Verification

Basic checks after build:

```bash
docker image inspect ${REGISTRY}/cdc-daemon:${VERSION}
docker image inspect ${REGISTRY}/cdc-bff:${VERSION}
docker image inspect ${REGISTRY}/cdc-ctl:${VERSION}
docker image inspect ${REGISTRY}/cdc-web-console:${VERSION}
```

Smoke run examples:

```bash
docker run --rm ${REGISTRY}/cdc-ctl:${VERSION} --help
```

## 9. CI/CD recommendations

- Build on every merge to main.
- Tag immutable release images (for example git SHA + semantic version).
- Generate SBOM and vulnerability scan reports.
- Sign images and enforce signature verification in cluster admission policy.
- Promote images across environments instead of rebuilding per environment.
