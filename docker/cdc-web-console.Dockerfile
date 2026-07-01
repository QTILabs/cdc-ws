# syntax=docker/dockerfile:1.7

# ── Stage 1: Install deps ──────────────────────────────────────
FROM node:22-alpine AS deps
WORKDIR /workspace/cdc-web-console

COPY cdc-web-console/package.json ./
COPY cdc-web-console/pnpm-lock.yaml* ./
COPY cdc-web-console/package-lock.json* ./

# Approve build scripts directly in package.json (pnpm v11+ blocks unknown)
# This avoids interactive `pnpm approve-builds` in Docker
RUN corepack enable && \
    echo 'const p=require("./package.json");p.pnpm=p.pnpm||{};p.pnpm.onlyBuiltDependencies=["@parcel/watcher","esbuild"];require("fs").writeFileSync("./package.json",JSON.stringify(p,null,2));' | node && \
    pnpm install --frozen-lockfile

# ── Stage 2: Build ──────────────────────────────────────────────
FROM node:22-alpine AS builder
WORKDIR /workspace/cdc-web-console

COPY --from=deps /workspace/cdc-web-console/node_modules ./node_modules
COPY cdc-web-console/ ./

ENV NODE_ENV=production
RUN npm run build

# ── Stage 3: Runtime ────────────────────────────────────────────
FROM node:22-alpine
WORKDIR /app

# node:22-alpine doesn't need libc — just run
COPY --from=builder /workspace/cdc-web-console/.output ./.output

EXPOSE 3000
ENV NODE_ENV=production

# SolidStart/Node server (not nginx)
CMD ["node", ".output/server/index.mjs"]
