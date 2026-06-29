# Engineering Specification: Multiplexed PostgreSQL to OpenSearch CDC Pipeline
**Component:** Custom Multi-Topic Rustls Sink Daemon via Native OpenSearch Driver  
**Telemetry Standard:** OpenTelemetry (OTel) Native Traces, Metrics, and Logs over OTLP  
**Target Environment:** Kubernetes (EKS/GKE)  

---

## 1. Architectural Overview

This system architecture establishes an enterprise-grade, low-latency Change Data Capture (CDC) pipeline shifting transactional mutations from a production PostgreSQL cluster into distinct target OpenSearch search indices. 

Instead of deploying heavy, resource-intensive intermediate message brokers (e.g., Kafka/Kafka Connect), the architecture leverages **RisingWave** as a cloud-native stream processor and a custom-built, highly concurrent **Rust Daemon** acting as the secure delivery vehicle.

All logs, context-aware span metrics, and transaction latency graphs are gathered seamlessly and dispatched via **OTLP gRPC network flows** to an upstream central collection infrastructure.

---

## 2. Telemetry Invariants & Core Instrumentation SLIs

The daemon registers and exposes five foundational metrics to your APM scraping engines, tagged explicitly by runtime configuration contexts:

| Metric Metric Name | Type Abstraction | Dimensions / Tags | Functional SLI Target Evaluated |
| :--- | :--- | :--- | :--- |
| `pipeline_records_ingested` | Counter | `subscription` | Ingestion throughput velocity mapping database extractions |
| `pipeline_records_sunk` | Counter | `target_index` | Effective index sink write velocity achieving finality |
| `pipeline_fetch_latency_seconds` | Histogram | `subscription` | Inbound network evaluation execution time profiling |
| `pipeline_flush_latency_seconds` | Histogram | `target_index` | Outbound OpenSearch HTTP bulk submission latency profiling |
| `pipeline_mpsc_available_capacity` | Observable Gauge | None | Buffer memory exhaustion tracking metric |

---

## 3. Upstream Infrastructure Configuration (RisingWave)

Execute the following Data Definition Language (DDL) specifications within the RisingWave cluster. This provisions the native CDC ingestion workers, establishes the localized state views, and defines the change log stream interfaces with explicit data retention.

Canonical runnable example location:

- `examples/risingwave/postgres-to-opensearch.sql`

The snippet below mirrors that file and should be kept in sync.

```sql
-- 1. Inbound Users Replication Source
CREATE TABLE src_postgres_users (
    user_id INT,
    username VARCHAR,
    email VARCHAR,
    PRIMARY KEY (user_id)
) WITH (
    connector = 'postgres-cdc',
    hostname = 'postgres-primary.database.svc.cluster.local',
    port = '5432',
    username = 'rw_cdc_user',
    password = 'your_secure_postgres_password',
    database.name = 'production',
    schema.name = 'public',
    table.name = 'users'
);

-- 2. Inbound Orders Replication Source
CREATE TABLE src_postgres_orders (
    order_id INT,
    user_id INT,
    total_amount NUMERIC,
    status VARCHAR,
    PRIMARY KEY (order_id)
) WITH (
    connector = 'postgres-cdc',
    hostname = 'postgres-primary.database.svc.cluster.local',
    port = '5432',
    username = 'rw_cdc_user',
    password = 'your_secure_postgres_password',
    database.name = 'production',
    schema.name = 'public',
    table.name = 'orders'
);

-- 3. Materialized Projections
CREATE MATERIALIZED VIEW mv_user_analytics AS 
SELECT user_id, username, email FROM src_postgres_users;

CREATE MATERIALIZED VIEW mv_order_analytics AS 
SELECT order_id, user_id, total_amount, status FROM src_postgres_orders;

-- 4. High-Performance Changelog Stream Subscriptions
CREATE SUBSCRIPTION sub_users FROM mv_user_analytics WITH (retention = '1D');
CREATE SUBSCRIPTION sub_orders FROM mv_order_analytics WITH (retention = '1D');
```

---

## 4. Build and Image Specification

Production images should be built from checked-in multi-stage Dockerfiles:

- `docker/cdc-daemon.Dockerfile`
- `docker/cdc-bff.Dockerfile`
- `docker/cdc-ctl.Dockerfile`
- `docker/cdc-web-console.Dockerfile`

Build posture requirements:

- compile Rust binaries in a builder stage and copy only release artifacts into runtime images
- keep runtime images minimal and include CA certificates for TLS trust
- use immutable tags (for example semantic version plus commit SHA), not mutable `latest`

---

## 5. Kubernetes Deployment Baseline

Starter deployment manifest:

- `k8s/cdc-starter.yaml`

Current baseline encoded in the starter manifest:

- namespace: `cdc`
- daemon deployment: `replicas: 1`, image `ghcr.io/your-org/cdc-daemon:0.1.0`
- BFF deployment: `replicas: 2`, image `ghcr.io/your-org/cdc-bff:0.1.0`
- readiness/liveness probes, resource requests and limits, and PodDisruptionBudget for both services

Operational note:

- keep the version tags in the manifest aligned with published image versions from CI/CD

---

## 6. Image Versioning and Promotion Policy

Use immutable versioning for all deployable images and promote the same built artifact across environments.

### 6.1 Tag format

Publish each image with at least:

- semantic version tag (example: `0.1.0`)
- commit tag (example: `sha-<git-sha>`)

Optional convenience tags for non-production discovery:

- branch tags (example: `main-<short-sha>`)

Do not rely on mutable tags such as `latest` for deployment manifests.

### 6.2 Environment promotion model

1. Build once in CI from a reviewed commit.
2. Push image with semantic and commit tags.
3. Deploy to dev using the commit tag or digest.
4. Promote to staging by reusing the same image digest.
5. Promote to production by reusing the same image digest.

The artifact digest must not change between environments.

### 6.3 Kubernetes pinning requirement

Preferred production pinning:

- `image: ghcr.io/your-org/cdc-daemon@sha256:<digest>`
- `image: ghcr.io/your-org/cdc-bff@sha256:<digest>`

If tags are temporarily used in lower environments, keep them immutable and map each tag to a recorded digest in release metadata.

### 6.4 Rollback policy

- keep at least the last 3 production image digests available
- rollback by updating workload image references to the previous known-good digest
- avoid rebuilding old commits for rollback; redeploy the stored digest

### 6.5 Release evidence

For each promoted version, retain:

- source commit SHA
- image digest per component
- SBOM and vulnerability scan result
- deployment change record (environment, timestamp, approver)