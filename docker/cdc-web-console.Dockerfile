# syntax=docker/dockerfile:1.7

FROM node:22-alpine AS builder
WORKDIR /workspace/cdc-web-console

COPY cdc-web-console/package.json cdc-web-console/pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile

COPY cdc-web-console ./
RUN pnpm build

FROM nginx:1.27-alpine
COPY --from=builder /workspace/cdc-web-console/dist /usr/share/nginx/html

EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
