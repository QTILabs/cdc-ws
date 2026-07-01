# syntax=docker/dockerfile:1.7

# ── Stage 1: Build FE ──────────────────────────────────────────
FROM node:22-alpine AS builder
WORKDIR /workspace

# Copy dependency manifests only (layer cache)
COPY cdc-web-console/package.json ./cdc-web-console/
COPY cdc-web-console/pnpm-lock.yaml* ./cdc-web-console/
COPY cdc-web-console/package-lock.json* ./cdc-web-console/

# Install: pnpm preferred, npm fallback
RUN cd cdc-web-console && \
    if [ -f pnpm-lock.yaml ]; then \
        corepack enable && pnpm install --frozen-lockfile; \
    else \
        npm ci; \
    fi

# Copy the rest of FE source
COPY cdc-web-console/ ./cdc-web-console/

# Build
RUN cd cdc-web-console && npm run build

# ── Stage 2: Nginx ─────────────────────────────────────────────
FROM nginx:1.27-alpine
COPY --from=builder /workspace/cdc-web-console/dist /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
