# syntax=docker/dockerfile:1.7

# =============================================================================
# CDC Web Console — SolidStart + PNPM Dockerfile
# =============================================================================
# Build:
#   docker build -f docker/cdc-web-console.Dockerfile -t cdc-web-console:latest .
#   docker compose build cdc-web-console
# =============================================================================

# ── Stage 1: Install dependencies & build ──────────────────────────────────
FROM node:22-alpine AS builder

# ── Pin pnpm version ─────────────────────────────────────────────────────
# Corepack reads packageManager field; COREPACK_INTEGRITY_KEYS=0 skips
# signature verification for speed (official npm registry is safe in Docker).
ENV COREPACK_ENABLE_STRICT=0 \
    COREPACK_INTEGRITY_KEYS=0

RUN corepack enable

# ── Build prerequisites ─────────────────────────────────────────────────
# esbuild and @parcel/watcher ship prebuilt binaries for linux-x64-musl
# (Alpine), so no python/g++ needed. Only add ca-certificates for HTTPS.
RUN apk add --no-cache ca-certificates

# ── PNPM store on cache mount ────────────────────────────────────────────
# Redirect pnpm store to a path we can mount as a BuildKit cache,
# so `pnpm install` across rebuilds only downloads changed packages.
RUN pnpm config set store-dir /app/.pnpm-store

WORKDIR /app

# ── Dependency layer (cached) ───────────────────────────────────────────
# Copy ONLY lockfile + manifest → pnpm install hits cache → layer is stable
# as long as lockfile doesn't change.
COPY cdc-web-console/pnpm-lock.yaml cdc-web-console/package.json ./

RUN --mount=type=cache,target=/app/.pnpm-store,sharing=locked \
    pnpm install --frozen-lockfile --prod=false

# ── Source layer ─────────────────────────────────────────────────────────
# Copy everything except what .dockerignore excludes.
COPY cdc-web-console/ ./

# ── Build ────────────────────────────────────────────────────────────────
ENV NODE_ENV=production
RUN --mount=type=cache,target=/app/.pnpm-store,sharing=locked \
    pnpm build

# ── Prune dev dependencies for smaller runtime ───────────────────────────
RUN --mount=type=cache,target=/app/.pnpm-store,sharing=locked \
    pnpm prune --prod --ignore-scripts

# ── Stage 2: Production runtime ──────────────────────────────────────────
FROM node:22-alpine

WORKDIR /app

# Only runtime essentials: compiled output + package.json (for metadata)
COPY --from=builder /app/.output ./.output
COPY --from=builder /app/package.json ./

EXPOSE 3000
ENV NODE_ENV=production \
    PORT=3000

# Use node directly (no Express, no nginx for FE in dev-style SSR)
CMD ["node", ".output/server/index.mjs"]

HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=15s \
    CMD wget -q -O /dev/null http://localhost:3000/ || exit 1
