# CDC Workspace — Dokumentasi Lengkap

> **Versi:** 1.0.0  
> **Tanggal:** 2026-06-30  
> **Tech Stack:** Rust, Axum, Tonic (gRPC), React + Vite, OpenSearch, RisingWave, PostgreSQL

---

## 📋 Daftar Isi

1. [Apa Itu CDC Workspace?](#1-apa-itu-cdc-workspace)
2. [Arsitektur Sistem](#2-arsitektur-sistem)
3. [Komponen & Fungsinya](#3-komponen--fungsinya)
4. [Alur Data End-to-End](#4-alur-data-end-to-end)
5. [Tech Stack](#5-tech-stack)
6. [Persiapan Sebelum Mulai](#6-persiapan-sebelum-mulai)
7. [Cara Menjalankan](#7-cara-menjalankan)
8. [Referensi Konfigurasi (.env)](#8-referensi-konfigurasi-env)
9. [Pipeline Configuration](#9-pipeline-configuration)
10. [API Reference](#10-api-reference)
11. [cdc-ctl — CLI Tool](#11-cdc-ctl--cli-tool)
12. [Troubleshooting](#12-troubleshooting)
13. [FAQ](#13-faq)

---

## 1. Apa Itu CDC Workspace?

**CDC Workspace** adalah sistem **Change Data Capture (CDC)** yang memindahkan data secara _real-time_ dari **RisingWave** (database streaming) ke **OpenSearch** (search & analytics engine).

### Kenapa CDC penting?

Di sistem tradisional, data dipindahkan secara _batch_ (misalnya tiap jam). CDC menangkap **setiap perubahan data** begitu terjadi — insert, update, delete — dan langsung mengirimkannya ke target. Hasilnya: data di OpenSearch selalu **up-to-date secara real-time**.

### Use Case

| Skenario | Manfaat |
|---|---|
| **Dashboard real-time** | Data transaksi langsung muncul di grafik |
| **Search & analytics** | OpenSearch bisa nge-index data dari RisingWave untuk full-text search |
| **Data replication** | Backup live dari satu DB ke sistem lain |
| **Event-driven architecture** | Setiap perubahan data bisa trigger action lain |

---

## 2. Arsitektur Sistem

```
┌─────────────────────────────────────────────────────┐
│                Browser / Client                      │
│           (React Web Console / curl / Postman)       │
└──────────────────────┬──────────────────────────────┘
                       │ HTTPS / JSON / JWT
                       ▼
┌──────────────────────────────────────────────────────┐
│              BFF Gateway (cdc-bff)                    │
│         Axum REST API — Port :8080                    │
│   ┌─────────────┐  ┌──────────────────────────────┐   │
│   │ Auth (/api/auth) │  │ CDC API (/api/cdc/*)      │   │
│   │ - Login local    │  │ - Health / Metrics        │   │
│   │ - OAuth2/GitHub  │  │ - List Pipelines          │   │
│   │ - Keycloak OIDC  │  │ - Reload Pipeline         │   │
│   │ - JWT signing    │  │ - OpenAPI/Swagger         │   │
│   └─────────────────┘  └──────────┬─────────────────┘   │
└──────────────────────────────────────┬──────────────────┘
                                       │ gRPC / Protobuf
                                       ▼
┌──────────────────────────────────────────────────────────┐
│               CDC Daemon (cdc-daemon)                    │
│          Tonic gRPC Server — Port :50051                 │
│                                                          │
│    ┌──────────────┐     ┌──────────────┐                 │
│    │  Producer     │────▶│  Consumer     │                │
│    │  (PostgreSQL) │     │  (OpenSearch) │                │
│    │  Baca dari    │     │  Tulis ke     │                │
│    │  subscription │     │  index OS     │                │
│    └──────┬───────┘     └──────┬─────────┘               │
│           │                    │                          │
│           ▼                    ▼                          │
│    ┌──────────────┐     ┌──────────────┐                 │
│    │  DLQ Manager  │     │  Metrics     │                 │
│    │  Dead Letter  │     │  (Atomik)    │                 │
│    │  Queue        │     │  + OTLP     │                 │
│    └──────────────┘     └──────────────┘                 │
└──────────────────────────────────────────────────────────┘
           │                          │
           ▼                          ▼
    ┌──────────────┐          ┌──────────────┐
    │  RisingWave   │          │  OpenSearch   │
    │  (Source DB)  │          │  (Sink)       │
    │  Port :4566   │          │  Port :9200   │
    └──────────────┘          └──────────────┘
```

### Alur Singkat

1. **User** → login via BFF (`/api/auth/login`) → dapat JWT
2. **User** → request data CDC (`/api/cdc/*`) dengan JWT
3. **BFF** → validasi JWT → forward ke **Daemon** via gRPC
4. **Daemon** → baca data dari **RisingWave subscription**
5. **Daemon** → kirim ke **OpenSearch** sebagai dokumen
6. **Daemon** → lapor metrics via **OTLP** (SigNoz)

---

## 3. Komponen & Fungsinya

### 3.1 cdc-daemon (Core Engine)

| Fungsi | Detail |
|---|---|
| **Produksi data** | Baca stream dari RisingWave subscription (PostgreSQL protocol) |
| **Konsumsi data** | Bulk insert/update ke OpenSearch |
| **Backfill** | Sinkronisasi data existing (MV) ke OpenSearch |
| **gRPC Server** | Serve endpoint health, metrics, list pipelines, pause |
| **DLQ** | Record gagal disimpan ke dead-letter queue |
| **Telemetry** | Metrics via OpenTelemetry OTLP |
| **Hot Reload** | Pipeline config bisa di-reload tanpa restart |

### 3.2 cdc-bff (Backend-for-Frontend / API Gateway)

| Fungsi | Detail |
|---|---|
| **REST API** | Axum server di port 8080 |
| **Auth** | Login local (username/password) + OAuth2 (GitHub) + Keycloak OIDC |
| **JWT** | Token signing & verifikasi lokal (sovereign) |
| **Swagger** | OpenAPI spec di `/api/openapi.json` |
| **CORS** | Allow frontend origin |
| **gRPC Client** | Proxy request ke daemon via Protobuf |

### 3.3 cdc-ctl (CLI Tool)

| Perintah | Fungsi |
|---|---|
| `start` | Start daemon (foreground/background) |
| `stop` | Stop daemon via gRPC |
| `reload` | Hot reload pipeline config |
| `print-config` | Lihat konfigurasi aktif (rahasia di-mask) |

### 3.4 cdc-web-console (Frontend)

| Fitur | Detail |
|---|---|
| **Dashboard** | Overview status CDC |
| **Pipeline List** | Lihat semua pipeline + status |
| **Login** | Form login + OAuth redirect |
| **Built with** | React + TypeScript + Vite |

---

## 4. Alur Data End-to-End

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌───────────┐
│RisingWave │────▶│  Daemon  │────▶│OpenSearch│────▶│Dashboard  │
│(Source)   │     │(Pipeline)│     │(Sink)    │     │(FE/Kibana)│
└──────────┘     └──────────┘     └──────────┘     └───────────┘
     │                │                │
     │ subscription   │                │
     │ sub_laporan_   │                │
     │ rw_master      │                │
                     │                │
                     │ batch 1000     │
                     │ id_field: id   │
```

**Detail langkah:**

1. **RisingWave** punya subscription (`sub_laporan_rw_master`) yang mencatat perubahan data dari tabel/MV
2. **Daemon Producer** konek ke RW via PostgreSQL protocol, fetch batch (1000 row) dari subscription
3. Data dikirim via channel ke **Consumer**
4. **Consumer** bulk index ke OpenSearch index `laporan_rw_master`
5. Kalau gagal → masuk **DLQ** (dead-letter queue) di disk
6. Metrics terekam via **OTLP** ke SigNoz

---

## 5. Tech Stack

| Layer | Teknologi | Versi |
|---|---|---|
| **Bahasa** | Rust | 1.95+ |
| **HTTP Framework** | Axum | 0.8 |
| **gRPC** | Tonic | 0.14 |
| **Serialization** | Protobuf + Prost | - |
| **Auth** | jsonwebtoken + oauth2 | - |
| **DB Source** | PostgreSQL/RisingWave | - |
| **DB Sink** | OpenSearch | - |
| **OpenAPI** | Utoipa | - |
| **Frontend** | React + TypeScript + Vite | - |
| **Observability** | OpenTelemetry (OTLP) | - |
| **Container** | Docker (multi-stage) | - |

---

## 6. Persiapan Sebelum Mulai

### Prasyarat

```bash
# 1. Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Protobuf compiler (wajib untuk compile gRPC)
brew install protobuf    # macOS
# apt install protobuf-compiler  # Ubuntu

# 3. Node.js (untuk FE)
# Download dari https://nodejs.org atau via brew
brew install node

# 4. (Opsional) pnpm — tapi Makefile otomatis fallback ke npm
npm install -g pnpm
```

### Cek Prasyarat

```bash
make protoc-check    # Cek protoc
cargo --version      # Cek Rust
node --version       # Cek Node (min 18)
```

---

## 7. Cara Menjalankan

### Cara 1: Start All (Build + Run) — Paling Gampang

```bash
make start-all
```

Ini akan:
1. Kill service yang mungkin masih jalan
2. Build semua komponen Rust (parallel)
3. Start daemon (background)
4. Start BFF (background)
5. Install & start FE (background)
6. Polling sampai semua service ready

Output:

```
═══ CDC Workspace — Semua Running ═══
  Daemon  : http://localhost:50051 (gRPC)
  BFF API : http://localhost:8080   (REST)
  FE      : http://localhost:5173   (UI)
  Login   : admin / admin_password
```

### Cara 2: Quick Start (Skip Build) — Kalau Binary Udah Ada

```bash
make q-all
```

Lebih cepat (~5 detik). Pakai binary yang sudah ter-compile di `target/debug/`.

### Cara 3: Satu per Satu

```bash
# Step by step
make kill              # Bersihin dulu
make build-daemon      # Build daemon
make build-bff         # Build BFF
make start-daemon      # Start daemon
make start-bff         # Start BFF
make fe-dev            # Start FE
```

### Cara 4: Via cdc-ctl

```bash
# Start daemon via CLI
make ctl-start

# Reload pipeline config setelah ubah pipelines.yaml
make ctl-reload

# Stop daemon via gRPC
make ctl-stop
```

### Cek Status

```bash
make status        # Cek semua port
make api-health    # Test health endpoint (auto login)
make api-pipelines # List pipeline (auto login)
make api-swagger   # Lihat OpenAPI paths
make logs          # Tail log daemon + BFF
```

### Stop Semua

```bash
make kill
```

---

## 8. Referensi Konfigurasi (.env)

File `.env` di root project. Ini konfigurasi yang dipakai:

### Source (RisingWave / Postgres)

| Variabel | Default | Wajib? | Fungsi |
|---|---|---|---|
| `RW_HOST` | `localhost` | ✅ | Host RisingWave |
| `RW_PORT` | `4566` | ✅ | Port RisingWave |
| `RW_USER` | `root` | ✅ | User |
| `RW_DBNAME` | `dev` | ✅ | Database name |
| `RW_SSLMODE` | `require` | - | SSL mode (`disable`, `require`, dll) |

### Sink (OpenSearch)

| Variabel | Default | Wajib? | Fungsi |
|---|---|---|---|
| `OS_URL` | `https://localhost:9200` | ✅ | OpenSearch endpoint |
| `OS_USER` | `admin` | ✅ | Username |
| `OS_PASSWORD` | - | ✅ **WAJIB** | Password |

### Telemetry

| Variabel | Default | Fungsi |
|---|---|---|
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | Endpoint OTLP collector (SigNoz) |

### Pipeline

| Variabel | Default | Fungsi |
|---|---|---|
| `PIPELINES_FILE` | `pipelines.yaml` | File definisi pipeline |
| `BACKFILL_ENABLED` | `false` | Aktifkan backfill (sink data existing) |

### Lainnya

| Variabel | Default | Fungsi |
|---|---|---|
| `CONSUMER_ID` | hostname | ID consumer |
| `HOSTNAME` | `local` | Label host |
| `LOCAL_DLQ_DIR` | `/var/log/cdc-dlq` | Directory DLQ |
| `CDC_DAEMON_GRPC_URL` | `http://localhost:50051` | URL gRPC daemon (untuk BFF) |
| `HEALTH_PORT` | `9090` | Port health check |

### Auth (di BFF)

| Variabel | Default | Fungsi |
|---|---|---|
| `JWT_SECRET` | `super_secret_key_change_me` | Secret signing JWT |
| `GITHUB_CLIENT_ID` | - | GitHub OAuth client ID |
| `GITHUB_CLIENT_SECRET` | - | GitHub OAuth secret |
| `KEYCLOAK_CLIENT_ID` | - | Keycloak client ID |
| `KEYCLOAK_CLIENT_SECRET` | - | Keycloak secret |

---

## 9. Pipeline Configuration

File `pipelines.yaml` di root project:

```yaml
- subscription_name: sub_laporan_rw_master
  target_index: laporan_rw_master
  id_field: id
  batch_size: 1000
```

| Field | Wajib | Fungsi |
|---|---|---|
| `subscription_name` | ✅ | Nama subscription di RisingWave |
| `target_index` | ✅ | Index tujuan di OpenSearch |
| `id_field` | ✅ | Field yang dipakai sebagai document ID |
| `batch_size` | - | Jumlah row per fetch (default: 500) |

**Cara ganti pipeline:**

```bash
# 1. Edit file pipelines.yaml
vim pipelines.yaml

# 2. Reload ke daemon (tanpa restart)
make ctl-reload

# 3. Verifikasi
make api-pipelines
```

---

## 10. API Reference

Base URL: `http://localhost:8080`

### Auth

| Method | Path | Auth | Fungsi |
|---|---|---|---|
| `POST` | `/api/auth/login` | ❌ | Login local → JWT |
| `GET` | `/api/auth/oauth2/github/login` | ❌ | Login via GitHub |
| `GET` | `/api/auth/oauth2/github/callback` | ❌ | Callback GitHub |
| `GET` | `/api/auth/oauth2/keycloak/login` | ❌ | Login via Keycloak |
| `GET` | `/api/auth/oauth2/keycloak/callback` | ❌ | Callback Keycloak |

### CDC (wajib JWT)

| Method | Path | Auth | Fungsi |
|---|---|---|---|
| `GET` | `/api/cdc/health` | ✅ JWT | Status daemon + pipeline |
| `GET` | `/api/cdc/metrics` | ✅ JWT | Metrics ingestion |
| `GET` | `/api/cdc/pipelines` | ✅ JWT | List pipeline + status |
| `POST` | `/api/cdc/pipelines/reload` | ✅ JWT | Reload pipeline config |

### Docs

| Method | Path | Auth | Fungsi |
|---|---|---|---|
| `GET` | `/api/openapi.json` | ❌ | OpenAPI spec (JSON) |

### Contoh Request

```bash
# Login
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin_password"}'

# Response:
# {"token":"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...","role":"admin"}

# Health check (dengan JWT)
TOKEN="eyJhbGciOiJIUzI1NiIs..."
curl http://localhost:8080/api/cdc/health \
  -H "Authorization: Bearer $TOKEN"

# Response:
# {"is_healthy":true,"overall_status":"RUNNING","components":{"sub_laporan_rw_master":"RUNNING"}}
```

---

## 11. cdc-ctl — CLI Tool

### Cara Pakai

```bash
# Via make
make ctl-print-config   # Lihat config
make ctl-reload         # Reload pipeline
make ctl-stop           # Stop daemon

# Langsung
cargo run -p cdc-ctl -- print-config
cargo run -p cdc-ctl -- reload --daemon-url http://localhost:50051
cargo run -p cdc-ctl -- stop --daemon-url http://localhost:50051
cargo run -p cdc-ctl -- start --foreground
```

### Subcommands

| Subcommand | Args | Fungsi |
|---|---|---|
| `start` | `--foreground`, `--daemon-bin` | Start daemon |
| `stop` | `--daemon-url` (default: `http://localhost:50051`) | Stop daemon |
| `reload` | `--daemon-url` | Hot reload pipeline |
| `print-config` | `--env-file`, `--pipelines-file` | Tampilkan config (rahasia di-mask) |

---

## 12. Troubleshooting

### ❌ Daemon & BFF mati setelah start-all

```bash
# Cek log
cat /tmp/cdc-daemon.log | tail -20
cat /tmp/cdc-bff.log | tail -20

# Restart manual
make kill
make q-all
```

### ❌ Build error: `Could not find protoc`

```bash
# Install protobuf
brew install protobuf   # macOS
apt install protobuf-compiler  # Linux

# Cek
protoc --version  # Harus ≥ 3.x
```

### ❌ BFF crash: `no CryptoProvider installed`

jsonwebtoken 10.x butuh crypto provider explicit:

```
# Tambahin di cdc-bff/cargo.toml
jsonwebtoken = { version = "10.4.0", features = ["rust_crypto"] }
```

### ❌ `sslmode=require` tapi koneksi PG/RW pakai non-TLS

Di `.env` set:

```env
RW_SSLMODE=disable
```

### ❌ Daemon log: `stream fetch failed; retrying`

Ini normal kalau subscription di RisingWave belum dibuat. Pipeline akan retry terus.

Buat subscription di RisingWave:

```sql
CREATE SUBSCRIPTION sub_laporan_rw_master 
  FROM mv_laporan_rw_master 
  WITH (copy_data = true);
```

### ❌ Login gagal (`401 Unauthorized`)

Cek `.env`:

```env
JWT_SECRET=isi_sama_seperti_di_bff_dan_daemon
```

### ❌ Port sudah dipakai

```bash
make kill-ports   # Paksa kill
```

### ❌ FE blank / broken

```bash
make kill-fe
make fe-install   # Reinstall deps
make fe-dev       # Start ulang
```

### ❌ Error `DepthLimitExceeded` di Swagger UI

Swagger UI default depth limit. Bisa dicurangi dengan paste URL langsung:  
`http://localhost:8080/api/openapi.json` → paste ke [petstore.swagger.io](https://petstore.swagger.io/)

---

## 13. FAQ

**Q: Apakah harus punya RisingWave & OpenSearch yang jalan?**  
A: Iya. Daemon konek ke RisingWave sebagai source & OpenSearch sebagai sink. Tanpa keduanya, daemon tetap jalan tapi log-nya `stream fetch failed`.

**Q: Bisa pakai PostgreSQL biasa, bukan RisingWave?**  
A: Secara teknis iya — daemon pakai PostgreSQL protocol. Tapi fitur subscription (CDC) spesifik ke RisingWave.

**Q: Gimana cara nambah pipeline baru?**  
A: Edit `pipelines.yaml`, tambah entry baru, jalankan `make ctl-reload`.

**Q: Data aman?**  
A: Ya. Auth pake JWT + OAuth2 PKCE. Password di `.env` jangan di-commit ke git. Sudah ada `.gitignore`.

**Q: Bisa production?**  
A: Project ini udah production-oriented — ada Docker multi-stage, Kubernetes manifest, OpenTelemetry, DLQ. Tapi tetap perlu disesuaikan dengan infra masing-masing (SSL, secrets management, dll).

**Q: Dimana log?**  
A: `/tmp/cdc-daemon.log`, `/tmp/cdc-bff.log`, `/tmp/cdc-fe.log`.  
Atau `make logs` untuk tail daemon + bff.

**Q: Gimana nge-reset semua?**  
A: `make kill && cargo clean && make start-all`

---

> 📘 **Catatan:** Dokumen ini untuk CDC Workspace di `/Volumes/Sinise/work/Agent/cdc-ws`.  
> Untuk info lebih teknis, lihat file-file berikut:
> - [architecture_walkthrough.md](architecture_walkthrough.md) — arsitektur & security
> - [daemon-configuration.md](daemon-configuration.md) — konfigurasi detail
> - [specs.md](specs.md) — spesifikasi teknis
> - [Makefile](Makefile) — semua command
