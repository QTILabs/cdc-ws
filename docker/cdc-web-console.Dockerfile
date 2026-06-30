# syntax=docker/dockerfile:1.7

# ── Stage 1: Build FE ──────────────────────────────────────────
FROM node:22-alpine AS builder
WORKDIR /workspace/cdc-web-console

# Copy dependency manifests only (layer cache)
COPY cdc-web-console/package.json ./
COPY cdc-web-console/pnpm-lock.yaml* ./
COPY cdc-web-console/package-lock.json* ./

# Install: pnpm preferred, npm fallback
RUN if [ -f pnpm-lock.yaml ]; then \
        corepack enable && pnpm install --frozen-lockfile; \
    else \
        npm ci; \
    fi

# Copy the rest of FE source
COPY cdc-web-console/ .

# Build
RUN npm run build

# ── Stage 2: Nginx ─────────────────────────────────────────────
FROM nginx:1.27-alpine
COPY --from=builder /workspace/cdc-web-console/dist /usr/share/nginx/html
COPY docker/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
