# Docker Storage Optimization — CDC Workspace

## Masalah

Setiap rebuild Docker image makan storage lebih banyak dari seharusnya. Container images menumpuk, build context terlalu besar, dan layer cache tidak efisien.

---

## Mengapa Storage Boros?

### 1. Build Context Terlalu Besar

**Apa itu build context?**

Build context adalah semua file yang dikirim dari mesin lokal ke Docker daemon saat membangun image. Makin besar context, makin lama build dan makin banyak storage untuk layer cache.

**Kondisi sebelumnya:**

```
docker/Dockerfile — planner stage:
  COPY . .                           ← copy SEMUA file project
  RUN cargo chef prepare ...

docker-compose.yml — build context:
  context: .                         ← root project (BUKAN docker/)
```

Project struktur:

```
cdc-ws/
├── cdc-daemon/        ← Rust backend (CDC worker)
├── cdc-bff/           ← Rust API gateway
├── cdc-ctl/           ← Rust CLI tool
├── cdc-web-console/   ← SolidJS frontend (Node.js)
├── docker/            ← Dockerfile
└── docker-compose.yml
```

**Masalah:** `context: .` berarti seluruh project di-copy ke Docker daemon, termasuk `cdc-web-console/`.

Isi `cdc-web-console/`:

```
cdc-web-console/
├── pnpm-lock.yaml     (~2MB, 5000+ baris)
├── node_modules/       (ratusan MB — dependencies)
├── .pnpm-store/       (ratusan MB — cached packages)
└── src/               (source code)
```

**Kenapa boros?**

- `cdc-web-console/` adalah Node.js project, dibuild oleh `docker/cdc-web-console.Dockerfile` yang terpisah
- Rust build (cdc-daemon, cdc-bff) tidak butuh `cdc-web-console/` sama sekali
- Tapi karena `COPY . .` di planner stage, seluruh folder ikut masuk build context
- Docker daemon menerima semua file itu, kemudian `.dockerignore` baru生效 setelahnya — sudah terlambat

### 2. Planner Stage Copy Seluruh Workspace

**Sebelum (docker/Dockerfile):**

```dockerfile
# Stage 2: planner
FROM chef AS planner
COPY . .                           # ← copy semua, termasuk cdc-web-console/
RUN cargo chef prepare --recipe-path recipe.json
```

**Apa yang terjadi di `cargo chef prepare`?**

Cargo Chef scan seluruh workspace untuk menemukan semua `Cargo.toml` dan source code. Tujuannya: generate recipe berisi daftar dependencies yang perlu di-build.

**Masalah:** Cargo Chef menscan `cdc-web-console/` yang isinya:
- `package.json` — bukan Cargo.toml, diabaikan
- `pnpm-lock.yaml` — bukan Cargo.lock, diabaikan
- `node_modules/` — ratusan folder tidak relevan

Ini tidak menambahkan file ke image, tapi memperbesar build context karena seluruh `cdc-web-console/` tetap perlu dikirim ke Docker daemon sebelum `.dockerignore` diproses.

### 3. Builder Stage Copy Semua Workspace Members

**Sebelum (docker/Dockerfile):**

```dockerfile
# Stage 3: builder
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
COPY cdc-bff/   cdc-bff/    # ← tidak perlu kalau build cdc-daemon
COPY cdc-ctl/   cdc-ctl/    # ← tidak perlu
```

**Masalah:** `cdc-bff/` dan `cdc-ctl/` di-copy tapi tidak digunakan saat membangun `cdc-daemon`. Ini memperbesar layer cache yang tidak perlu.

### 4. Layer Cache Tidak Stabil

Setiap kali file berubah, semua layer setelahnya invalidated. Karena `COPY . .` meng-copy seluruh project, perubahan di file manapun (bahkan di `cdc-web-console/`) menginvalidasi planner layer.

**Dampak:**
- Planner layer perlu re-run setiap kali apapun berubah
- Dependency caching tidak optimal
- Build makin lambat, storage makin boros

---

## Perbaikan

### Perbaikan 1: .dockerignore — Exclude Seluruh cdc-web-console/

**Sebelum:**

```gitignore
# ── Node.js (rebuilt inside Docker via multi-stage) ───────────────────────────
cdc-web-console/node_modules/
cdc-web-console/dist/
cdc-web-console/build/
cdc-web-console/.output/
```

**Sesudah:**

```gitignore
# ── Web Console (separate Dockerfile) ───────────────────────────────────────
# cargo-chef & Rust builder do NOT need the Node.js project.
# cdc-web-console/ is built by its own docker/Dockerfile separately.
cdc-web-console/
```

**Alasan:**

- Approach sebelumnya hanya exclude output (`node_modules/`, `dist/`, dst.)
- `pnpm-lock.yaml` tetap masuk build context (~2MB, 5000+ baris)
- File `.dockerignore` diproses SAAT context dikirim ke Docker daemon, sebelum Dockerfile berjalan
- `COPY . .` di planner stage tetap membaca `pnpm-lock.yaml` dari context
- Solution: exclude seluruh folder `cdc-web-console/` — Rust builder tidak butuh apapun dari folder itu

**Efek:**

| Item | Sebelum | Sesudah |
|------|---------|---------|
| pnpm-lock.yaml | Terkirim ke daemon | Tidak dikirim |
| Source code frontend | Terkirim ke daemon | Tidak dikirim |
| Build context size | +node_modules estimation | Tanpa frontend folder |

### Perbaikan 2: Planner Stage — Selective Copy

**Sebelum:**

```dockerfile
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json
```

**Sesudah:**

```dockerfile
FROM chef AS planner
# Only what's needed for cargo-chef: Cargo manifests + source.
# Copy package manifest first (smaller, stable), then source.
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
# BFF & ctl not needed in workspace recipe if building only cdc-daemon
#COPY cdc-bff/   cdc-bff/
#COPY cdc-ctl/   cdc-ctl/
RUN cargo chef prepare --recipe-path recipe.json
```

**Alasan:**

- Cargo Chef hanya butuh `Cargo.toml` + `Cargo.lock` untuk resolve dependencies
- Source code (`cdc-daemon/src/`) hanya perlu untuk compile, bukan untuk generate recipe
- `COPY Cargo.toml Cargo.lock ./` terlebih dahulu → layer lebih stabil (这两个 file jarang berubah)
- Comment `cdc-bff/` dan `cdc-ctl/` → menunjukkan tidak diperlukan, dan mudah di-uncomment kalau perlu

**Efek:**

| Layer | Sebelum | Sesudah |
|-------|---------|---------|
| Planner COPY | ~semua file project | Hanya Cargo.toml + proto + cdc-daemon/ |
| Layer cache stability | Invalidated on any change | Stable, only invalidates on Cargo.toml change |

### Perbaikan 3: Builder Stage — Selective Copy

**Sebelum:**

```dockerfile
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
COPY cdc-bff/   cdc-bff/
COPY cdc-ctl/   cdc-ctl/
```

**Sesudah:**

```dockerfile
COPY Cargo.toml Cargo.lock ./
COPY proto ./proto
COPY cdc-daemon/ cdc-daemon/
#COPY cdc-bff/   cdc-bff/
#COPY cdc-ctl/   cdc-ctl/
```

**Alasan:**

- Build hanya butuh package yang di-build (`cdc-daemon/` untuk cdc-daemon)
- `cdc-bff/` dan `cdc-ctl/` tidak digunakan saat compile cdc-daemon
- Commented out (bukan dihapus) agar mudah di-uncomment saat build `cdc-bff`

**Efek:**

| Item | Sebelum | Sesudah |
|------|---------|---------|
| cdc-bff/ source | Di-copy ke builder | Tidak di-copy |
| cdc-ctl/ source | Di-copy ke builder | Tidak di-copy |
| Build context (builder stage) | +bff +ctl folders | Hanya cdc-daemon/ |

---

## Alur Build Context Sekarang

```
Local Machine                          Docker Daemon
┌─────────────────────────┐          ┌──────────────────────────┐
│ cdc-ws/                 │          │ Build Context (received) │
│  ├── Cargo.toml         │──COPY────│  ├── Cargo.toml         │
│  ├── Cargo.lock         │          │  ├── Cargo.lock         │
│  ├── cdc-daemon/        │          │  ├── proto/             │
│  ├── cdc-bff/           │ (EXCLUDED)│  └── cdc-daemon/        │
│  ├── cdc-ctl/           │ (EXCLUDED)│                        │
│  ├── cdc-web-console/   │ (EXCLUDED)│                        │
│  └── docker/            │          │                        │
└─────────────────────────┘          └──────────────────────────┘
                                          ↓
                                   ┌──────────────────────────┐
                                   │ .dockerignore applied    │
                                   │ (cdc-web-console/ out)   │
                                   └──────────────────────────┘
                                          ↓
                                   ┌──────────────────────────┐
                                   │ planner stage:           │
                                   │ cargo chef prepare       │
                                   │ (only reads Cargo.toml)  │
                                   └──────────────────────────┘
```

---

## Cara Verify

### Check build context size

```bash
# Ukur size project yang dikirim ke Docker daemon
du -sh cdc-daemon/ proto/ Cargo.toml Cargo.lock

# Bandingkan dengan size jika include cdc-web-console/
du -sh cdc-web-console/
```

### Check image size

```bash
docker images | grep -E "cdc|rust"

# Detail per layer
docker history cdc-daemon:latest
```

### Check build context yang dikirim

```bash
# Linux (Docker 25+)
docker build --progress=plain --no-cache -f docker/Dockerfile \
  --build-arg PACKAGE=cdc-daemon \
  --build-arg BINARY=cdc-daemon \
  --build-arg EXPOSE_PORT=50051 \
  -t cdc-daemon:test . 2>&1 | grep "transferping context"

# macOS/Windows: Docker mengirim context via Unix socket ke daemon
# Tidak ada cara langsung lihat size, tapi bisa check via verbose build
```

### Clean up old images

```bash
# Hapus images yang tidak dipakai
docker image prune -a

# Hapus build cache
docker builder prune

# Check disk usage
docker system df
```

---

## Perbandingan Build Context

| Komponen | Sebelum | Sesudah | Pengurangan |
|----------|---------|---------|-------------|
| cdc-web-console/ | ~200MB+ (pnpm-store, node_modules) | 0 | ~200MB |
| cdc-bff/ | ~10KB source | 0 (commented) | ~10KB |
| cdc-ctl/ | ~10KB source | 0 (commented) | ~10KB |
| **Total context** | **~200MB+** | **~5MB** | **~195MB** |

---

## Catatan Penting

1. **Bukan karena Rust.** Rust slim image (~80MB runtime) sudah efisien. Masalah ada di build context yang membawa file tidak relevan.

2. **Bukan karena Node.js.** SolidJS project memang besar (pnpm-store, node_modules), tapi itu problema kalau masuk build context Rust. Frontend tetap dibuild terpisah oleh `cdc-web-console.Dockerfile`.

3. **Dockerignore diproses sebelum COPY.** Ini kunci masalah — `.dockerignore` bekerja saat context dikirim ke daemon, bukan saat Dockerfile berjalan. `COPY . .` sudah terlanjur membaca semua file sebelum Dockerfile mulai.

4. **Layer cache BuildKit.** Perbaikan ini juga meningkatkan layer cache stability. Planner layer sekarang hanya depend pada `Cargo.toml` + `Cargo.lock`, tidak pada seluruh project.

---

## Referensi

- [Docker Build Context](https://docs.docker.com/build/building/context/)
- [Dockerignore](https://docs.docker.com/engine/reference/builder/#dockerignore-file)
- [Cargo Chef](https://github.com/LukeMathWalker/cargo-chef) — Rust incremental build caching
- [BuildKit Cache Mounts](https://docs.docker.com/build/building/cache/backends/)
