# syntax=docker/dockerfile:1.7

# ── Stage 1: Install deps + build ───────────────────────────
FROM node:22-alpine AS builder

RUN corepack enable \
    && apk add --no-cache python3 make g++

WORKDIR /app

# Copy only manifests for dependency caching
COPY cdc-web-console/package.json cdc-web-console/pnpm-lock.yaml ./

# Install all dependencies (build scripts approved via package.json)
RUN pnpm install --frozen-lockfile

# Copy full source
COPY cdc-web-console/ ./

# Build
ENV NODE_ENV=production
RUN pnpm build

# Prune dev deps for smaller runtime
RUN pnpm prune --prod --ignore-scripts

# ── Stage 2: Production runtime ─────────────────────────────
FROM node:22-alpine
WORKDIR /app

COPY --from=builder /app/.output ./.output
COPY --from=builder /app/package.json ./

EXPOSE 3000
ENV NODE_ENV=production

CMD ["node", ".output/server/index.mjs"]
