# Changelog — Bug Fix 2026-07-03

## Permasalahan

**1. FE gagal start** — Vite dev server jalan tapi Makefile menganggap gagal.
**2. Daemon error `postgres fetch query failed: db error`** — Tiap detik spam WARN, pipeline stuck.
**3. Daemon error `sql parser error: expected CURSOR, found: -`** — Cursor name gak valid.

---

## Perbaikan

### 1. Vite port diselaraskan (`cdc-web-console/vite.config.ts`)
`server.port` dari `3000` → `5174`. Makefile cek port `5174`, jadi mismatch bikin false negative "FE gagal start" padahal server jalan di port lain.

### 2. Daemon load environment variable (`Makefile`)
Semua target daemon (`start-daemon`, `q-daemon`, `start-all`) sekarang inject `export $(grep -v '#' .env | xargs)` sebelum eksekusi binary. Sebelumnya `.env` cuma dibaca `dotenvy::dotenv()` di Rust, tapi daemon binary dipanggil via `cargo run` atau `./target/debug/...` — branch `dotenv().ok()` pernah gagal silent. Dengan export explicit di shell, env vars (`RW_HOST`, `RW_PORT`, dll) pasti terisi.

### 3. Cursor name di-quote (`cdc-daemon/src/main.rs`, `postgres.rs`)
Cursor name format `"subscription_consumer_id"` (dengan quote ganda). Sebelumnya `cursor_subscription_consumer_id` tanpa quote — `consumer_id` berisi hypen (`cdc-worker-1`), dan hypen adalah karakter illegal di identifier PostgreSQL tanpa quote.

### 4. Pipeline subscription name (`pipelines.yaml`)
`subscription_name` dari `sub_laporan_rw_master` → `sub_laporan_rw_master_dup`. Nama lama sudah tidak ada di RisingWave.

---

## File yang diubah

| File | Perubahan |
|------|-----------|
| `cdc-daemon/src/main.rs` | Cursor name quoted (line 312) |
| `cdc-daemon/src/postgres.rs` | Cursor name quoted + comment (line 110) |
| `cdc-web-console/vite.config.ts` | Port 3000 → 5174 |
| `Makefile` | Semua start target inject env vars dari `.env` |
| `pipelines.yaml` | Subscription name → `_dup` |
