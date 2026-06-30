# CDC Workspace — Local Makefile
# =============================================================================
# Usage: make <target>
# Run `make help` to see all targets
# =============================================================================

export CDC_BE_PORT  ?= 8080
export CDC_GRPC_PORT ?= 50051
export CDC_FE_PORT  ?= 5174
export CDC_ROOT     ?= /Volumes/Sinise/work/Agent/cdc-ws
export FE_DIR       ?= $(CDC_ROOT)/cdc-web-console
export LOG_DIR      ?= /tmp

.PHONY: help
help:
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort \
		| awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-22s\033[0m %s\n", $$1, $$2}'

# =============================================================================
# 1) KILL — Bersihin port & proses lama
# =============================================================================
.PHONY: kill kill-daemon kill-bff kill-fe kill-all kill-ports

kill: kill-all ## Stop semua service (daemon + bff + fe)

kill-all: kill-daemon kill-bff kill-fe ## Stop semua
	@echo "✅ Semua service dihentikan"

kill-daemon: ## Stop cdc-daemon
	@pkill -f "cdc-daemon" 2>/dev/null || true
	@echo "  daemon ✗"

kill-bff: ## Stop cdc-bff
	@pkill -f "cdc-bff" 2>/dev/null || true
	@echo "  bff ✗"

kill-fe: ## Stop FE (vite)
	@pkill -f "vite.*cdc-web-console" 2>/dev/null || true
	@echo "  fe ✗"

kill-ports: ## Paksa kill semua proses di port CDC
	@lsof -ti:$(CDC_GRPC_PORT) | xargs kill -9 2>/dev/null || true
	@lsof -ti:$(CDC_BE_PORT)   | xargs kill -9 2>/dev/null || true
	@lsof -ti:$(CDC_FE_PORT)   | xargs kill -9 2>/dev/null || true
	@echo "✅ Port $(CDC_GRPC_PORT), $(CDC_BE_PORT), $(CDC_FE_PORT) dibersihkan"

# =============================================================================
# 2) BUILD — Compile semua Rust components
# =============================================================================
.PHONY: build build-daemon build-bff build-ctl build-release

build: build-daemon build-bff build-ctl ## Build semua Rust components
	@echo "✅ Build selesai"

build-daemon: ## Build cdc-daemon
	@echo "  Building daemon..."
	cd $(CDC_ROOT) && cargo build -p cdc-daemon 2>&1

build-bff: ## Build cdc-bff (termasuk Swagger/OpenAPI)
	@echo "  Building bff..."
	cd $(CDC_ROOT) && cargo build -p cdc-bff 2>&1

build-ctl: ## Build cdc-ctl
	@echo "  Building ctl..."
	cd $(CDC_ROOT) && cargo build -p cdc-ctl 2>&1

build-release: ## Build semua (release mode)
	cd $(CDC_ROOT) && cargo build --release -p cdc-daemon -p cdc-bff -p cdc-ctl
	@echo "✅ Release build selesai"

# =============================================================================
# 3) FE — Frontend
# =============================================================================
.PHONY: fe-install fe-dev fe-build fe-preview

fe-install: ## Install FE dependencies (pnpm → npm fallback)
	cd $(FE_DIR) && (pnpm install 2>/dev/null || npm install)

fe-dev: ## Start FE dev server (background)
	@echo "  Starting FE on :$(CDC_FE_PORT)..."
	cd $(FE_DIR) && (pnpm dev 2>/dev/null || npm run dev -- --host --port $(CDC_FE_PORT)) \
		> $(LOG_DIR)/cdc-fe.log 2>&1 &
	@sleep 3
	@lsof -i:$(CDC_FE_PORT) >/dev/null 2>&1 \
		&& echo "  ✅ FE http://localhost:$(CDC_FE_PORT)" \
		|| echo "  ❌ FE gagal start, cek $(LOG_DIR)/cdc-fe.log"

fe-build: ## Build FE untuk production
	cd $(FE_DIR) && (pnpm build || npm run build)

# =============================================================================
# 4) START — Jalankan service (background)
# =============================================================================
.PHONY: start-daemon start-bff start-fe start-all restart

start-daemon: kill-daemon build-daemon ## Build & start cdc-daemon (background)
	@echo "  Starting daemon..."
	cd $(CDC_ROOT) && cargo run -p cdc-daemon > $(LOG_DIR)/cdc-daemon.log 2>&1 &
	@echo "  Menunggu daemon siap..."
	@for i in 1 2 3 4 5; do \
		lsof -i:$(CDC_GRPC_PORT) >/dev/null 2>&1 && break; \
		sleep 2; \
	done
	@lsof -i:$(CDC_GRPC_PORT) >/dev/null 2>&1 \
		&& echo "  ✅ Daemon :$(CDC_GRPC_PORT)" \
		|| echo "  ⏳ Daemon masih loading... cek $(LOG_DIR)/cdc-daemon.log"

start-bff: kill-bff build-bff ## Build & start cdc-bff (background)
	@echo "  Starting BFF..."
	cd $(CDC_ROOT) && cargo run -p cdc-bff > $(LOG_DIR)/cdc-bff.log 2>&1 &
	@echo "  Menunggu BFF siap..."
	@for i in 1 2 3 4 5 6 7 8; do \
		lsof -i:$(CDC_BE_PORT) >/dev/null 2>&1 && break; \
		sleep 2; \
	done
	@lsof -i:$(CDC_BE_PORT) >/dev/null 2>&1 \
		&& echo "  ✅ BFF http://localhost:$(CDC_BE_PORT)" \
		|| echo "  ❌ BFF gagal start, cek $(LOG_DIR)/cdc-bff.log"

start-fe: fe-install fe-dev ## Install & start FE
	@echo "  ✅ FE fitur sudah include di fe-dev"

start-all: kill-all
	@echo "=== Starting all services in parallel ==="
	@echo "  Building daemon + bff in background..."
	cd $(CDC_ROOT) && \
		cargo build -p cdc-daemon > /tmp/cdc-build.log 2>&1 & \
		cargo build -p cdc-bff  >> /tmp/cdc-build.log 2>&1 & \
		cargo build -p cdc-ctl  >> /tmp/cdc-build.log 2>&1 & \
		wait
	@echo "  ✅ Build selesai, start services..."
	@echo "  Starting daemon on :$(CDC_GRPC_PORT)..."
	cd $(CDC_ROOT) && ./target/debug/cdc-daemon > $(LOG_DIR)/cdc-daemon.log 2>&1 &
	@echo "  Starting BFF on :$(CDC_BE_PORT)..."
	cd $(CDC_ROOT) && ./target/debug/cdc-bff   > $(LOG_DIR)/cdc-bff.log   2>&1 &
	@echo "  Starting FE on :$(CDC_FE_PORT)..."
	cd $(FE_DIR) && (pnpm dev 2>/dev/null || npm run dev -- --host --port $(CDC_FE_PORT)) \
		> $(LOG_DIR)/cdc-fe.log 2>&1 &
	@echo ""
	@echo "  Menunggu services ready..."
	@for i in 1 2 3 4 5 6 7 8 9 10; do \
		d=$$(lsof -i:$(CDC_GRPC_PORT) >/dev/null 2>&1 && echo "✅" || echo "⏳"); \
		b=$$(lsof -i:$(CDC_BE_PORT)   >/dev/null 2>&1 && echo "✅" || echo "⏳"); \
		f=$$(lsof -i:$(CDC_FE_PORT)   >/dev/null 2>&1 && echo "✅" || echo "⏳"); \
		echo "    daemon:$$d  bff:$$b  fe:$$f"; \
		[ "$$d" = "✅" ] && [ "$$b" = "✅" ] && [ "$$f" = "✅" ] && break; \
		sleep 2; \
	done
	@echo ""
	@echo "═══════════════════════════════════════════"
	@echo " ✅  CDC Workspace  —  Semua Running"
	@echo "═══════════════════════════════════════════"
	@echo "  Daemon  : http://localhost:$(CDC_GRPC_PORT) (gRPC)"
	@echo "  BFF API : http://localhost:$(CDC_BE_PORT)   (REST)"
	@echo "  FE      : http://localhost:$(CDC_FE_PORT)   (UI)"
	@echo "  API Docs: http://localhost:$(CDC_BE_PORT)/api/openapi.json"
	@echo "═══════════════════════════════════════════"
	@echo "  Login: admin / admin_password"
	@echo "  Logs : $(LOG_DIR)/cdc-{daemon,bff,fe}.log"
	@echo "═══════════════════════════════════════════"

restart: kill-all start-all ## Restart semua service

# =============================================================================
# 5) QUICK — Skip rebuild (binary sudah ada)
# =============================================================================
.PHONY: q-daemon q-bff q-fe q-all

q-daemon: kill-daemon ## Start daemon cepat (skip build)
	@echo "  Starting daemon (cached binary)..."
	cd $(CDC_ROOT) && ./target/debug/cdc-daemon > $(LOG_DIR)/cdc-daemon.log 2>&1 &
	@sleep 3
	@lsof -i:$(CDC_GRPC_PORT) >/dev/null 2>&1 \
		&& echo "  ✅ Daemon :$(CDC_GRPC_PORT)" \
		|| echo "  ⏳ Daemon masih loading..."

q-bff: kill-bff ## Start BFF cepat (skip build)
	@echo "  Starting BFF (cached binary)..."
	cd $(CDC_ROOT) && ./target/debug/cdc-bff > $(LOG_DIR)/cdc-bff.log 2>&1 &
	@sleep 4
	@lsof -i:$(CDC_BE_PORT) >/dev/null 2>&1 \
		&& echo "  ✅ BFF http://localhost:$(CDC_BE_PORT)" \
		|| echo "  ❌ BFF gagal, cek $(LOG_DIR)/cdc-bff.log"

q-all: kill-all q-daemon q-bff fe-dev ## Start semua cepat (skip build)

# =============================================================================
# 6) CTL — CLI Control Tool
# =============================================================================
.PHONY: ctl-reload ctl-print-config ctl-stop ctl-start

ctl-print-config: build-ctl ## Print pipeline config
	cd $(CDC_ROOT) && cargo run -p cdc-ctl -- print-config

ctl-reload: build-ctl ## Hot reload pipeline (tanpa restart daemon)
	cd $(CDC_ROOT) && cargo run -p cdc-ctl -- reload \
		--daemon-url http://localhost:$(CDC_GRPC_PORT)

ctl-stop: build-ctl ## Stop daemon via gRPC
	cd $(CDC_ROOT) && cargo run -p cdc-ctl -- stop \
		--daemon-url http://localhost:$(CDC_GRPC_PORT)

ctl-start: build-ctl ## Start daemon via ctl (foreground)
	cd $(CDC_ROOT) && cargo run -p cdc-ctl -- start --foreground

# =============================================================================
# 7) STATUS & LOGS
# =============================================================================
.PHONY: status logs log-daemon log-bff log-fe

status: ## Cek status semua service
	@echo "=== Service Status ==="
	@for port in $(CDC_GRPC_PORT) $(CDC_BE_PORT) $(CDC_FE_PORT); do \
		name=""; \
		[ $$port = $(CDC_GRPC_PORT) ] && name="Daemon (gRPC)"; \
		[ $$port = $(CDC_BE_PORT) ] && name="BFF (REST)"; \
		[ $$port = $(CDC_FE_PORT) ] && name="FE (Vite)"; \
		lsof -i:$$port >/dev/null 2>&1 \
			&& echo "  ✅ $$name :$$port" \
			|| echo "  ❌ $$name :$$port — DOWN"; \
	done

logs: ## Tail semua log (Ctrl+C untuk stop)
	@tail -f $(LOG_DIR)/cdc-daemon.log $(LOG_DIR)/cdc-bff.log

log-daemon: ## Tail daemon log
	@tail -f $(LOG_DIR)/cdc-daemon.log

log-bff: ## Tail BFF log
	@tail -f $(LOG_DIR)/cdc-bff.log

log-fe: ## Tail FE log
	@tail -f $(LOG_DIR)/cdc-fe.log

# =============================================================================
# 8) API TEST
# =============================================================================
.PHONY: api-login api-health api-pipelines api-swagger

api-login: ## Login & dapatkan JWT token
	@echo "=== Login ==="
	@curl -s -X POST http://localhost:$(CDC_BE_PORT)/api/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username":"admin","password":"admin_password"}' | python3 -m json.tool

api-health: ## Health check (perlu login dulu — auto)
	@TOKEN=$$(curl -s -X POST http://localhost:$(CDC_BE_PORT)/api/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username":"admin","password":"admin_password"}' \
		| python3 -c "import sys,json; print(json.load(sys.stdin)['token'])"); \
	curl -s http://localhost:$(CDC_BE_PORT)/api/cdc/health \
		-H "Authorization: Bearer $${TOKEN}" | python3 -m json.tool

api-pipelines: ## List pipelines
	@TOKEN=$$(curl -s -X POST http://localhost:$(CDC_BE_PORT)/api/auth/login \
		-H "Content-Type: application/json" \
		-d '{"username":"admin","password":"admin_password"}' \
		| python3 -c "import sys,json; print(json.load(sys.stdin)['token'])"); \
	curl -s http://localhost:$(CDC_BE_PORT)/api/cdc/pipelines \
		-H "Authorization: Bearer $${TOKEN}" | python3 -m json.tool

api-swagger: ## Tampilkan OpenAPI spec
	@curl -s http://localhost:$(CDC_BE_PORT)/api/openapi.json \
		| python3 -c "import sys,json; d=json.load(sys.stdin); \
		print('Paths:', '\\n  '.join(d.get('paths',{}).keys()))"

# =============================================================================
# 9) UTILITIES
# =============================================================================
.PHONY: clean clean-all protoc-check

clean: ## Bersihin target Rust
	cd $(CDC_ROOT) && cargo clean

protoc-check: ## Cek protoc terinstall
	@which protoc && protoc --version || echo "❌ protoc tidak terinstall. Jalankan: brew install protobuf"

info: ## Info project
	@echo "=== CDC Workspace ==="
	@echo "Components:"
	@echo "  cdc-daemon — CDC core (gRPC, RW → OS sink)"
	@echo "  cdc-bff    — REST API (Axum, JWT/OAuth + Swagger)"
	@echo "  cdc-ctl    — CLI tool"
	@echo "  cdc-web-console — React FE"
	@echo ""
	@echo "Quick:"
	@echo "  make kill        — Stop semua"
	@echo "  make start-all   — Build & start semua"
	@echo "  make q-all       — Start cepat (skip build)"
	@echo "  make status      — Cek service"

# =============================================================================
# 10) DOCKER
# =============================================================================
# Build context  : project root (cdc-ws/)
# Compose file   : docker-compose.yml
# Images        : cdc-daemon, cdc-bff, cdc-web-console, cdc-ctl
# Compose ports  : daemon :50051, bff :8080, fe :5174
# =============================================================================

.PHONY: docker-build docker-build-daemon docker-build-bff docker-build-fe docker-build-ctl

docker-build: ## Build semua Docker image
	@echo "  Building cdc-daemon..."
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-daemon.Dockerfile -t cdc-daemon:latest $(CDC_ROOT)
	@echo "  Building cdc-bff..."
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-bff.Dockerfile -t cdc-bff:latest $(CDC_ROOT)
	@echo "  Building cdc-web-console..."
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-web-console.Dockerfile -t cdc-web-console:latest $(CDC_ROOT)
	@echo "  Building cdc-ctl..."
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-ctl.Dockerfile -t cdc-ctl:latest $(CDC_ROOT)
	@echo "✅ Semua image built"

docker-build-daemon: ## Build cdc-daemon image
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-daemon.Dockerfile -t cdc-daemon:latest $(CDC_ROOT)

docker-build-bff: ## Build cdc-bff image
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-bff.Dockerfile -t cdc-bff:latest $(CDC_ROOT)

docker-build-fe: ## Build cdc-web-console image
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-web-console.Dockerfile -t cdc-web-console:latest $(CDC_ROOT)

docker-build-ctl: ## Build cdc-ctl image
	docker build --build-arg BUILDKIT_INLINE_CACHE=1 \
		-f $(CDC_ROOT)/docker/cdc-ctl.Dockerfile -t cdc-ctl:latest $(CDC_ROOT)

# ── Compose ───────────────────────────────────────────────────────────────────
.PHONY: docker-up docker-up-build docker-down docker-logs docker-restart docker-ps

docker-env-check:
	@if [ ! -f $(CDC_ROOT)/.env ]; then \
		echo "❌ .env tidak ditemukan. Copy dulu: cp .env.example .env"; \
		exit 1; \
	fi
	@echo "  .env OK"

docker-up: docker-env-check ## Start semua service via docker-compose (pakai image existing)
	cd $(CDC_ROOT) && docker compose up -d
	@echo ""
	@echo "✅ Services up (docker):"
	@echo "  BFF API  : http://localhost:8080"
	@echo "  FE UI    : http://localhost:5174"
	@echo "  Daemon   : localhost:50051 (gRPC)"
	@echo "  Swagger  : http://localhost:8080/api/openapi.json"
	@echo ""
	@echo "  Login: admin / admin_password"

docker-up-build: docker-env-check docker-build docker-up ## Build image dulu, lalu start

docker-down: ## Stop & remove containers
	cd $(CDC_ROOT) && docker compose down

docker-logs: ## Tail log semua service
	cd $(CDC_ROOT) && docker compose logs -f

docker-logs-daemon: ## Tail daemon log
	cd $(CDC_ROOT) && docker compose logs -f cdc-daemon

docker-logs-bff: ## Tail BFF log
	cd $(CDC_ROOT) && docker compose logs -f cdc-bff

docker-logs-fe: ## Tail FE log
	cd $(CDC_ROOT) && docker compose logs -f cdc-web-console

docker-ps: ## Show running containers
	cd $(CDC_ROOT) && docker compose ps

docker-restart: docker-down docker-up ## Restart semua service

# ── Push ───────────────────────────────────────────────────────────────────────
.PHONY: docker-push
docker-push: ## Push images (set IMAGE_REGISTRY=docker.io/username)
	docker push $(IMAGE_REGISTRY)/cdc-daemon:latest
	docker push $(IMAGE_REGISTRY)/cdc-bff:latest
	docker push $(IMAGE_REGISTRY)/cdc-web-console:latest
