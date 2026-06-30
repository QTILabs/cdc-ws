# CDC Workspace Development Guide

This file is now the quickstart and index. Canonical operational content is consolidated into three primary docs:

1. [architecture-walkthrough.md](architecture-walkthrough.md): architecture, control plane, security model, and enterprise operating baseline
2. [daemon-configuration.md](daemon-configuration.md): runtime configuration, environment variables, cdc-ctl operations, and troubleshooting
3. [image-generation-guide.md](image-generation-guide.md): Dockerfiles, image build/push workflow, and CI/CD image practices

## Quickstart

1. Configure runtime values in `.env` and pipeline definitions in `pipelines.yaml`.
2. Start daemon:

```bash
cargo run -p cdc-ctl -- start --foreground
```

3. Check daemon status:

```bash
cargo run -p cdc-ctl -- status
```

4. Start BFF:

```bash
cargo run -p cdc-bff
```

5. Start web console:

```bash
cd cdc-web-console
pnpm install
pnpm run dev
```

The dashboard includes aggregate counters and per-sink metrics from `/api/cdc/metrics`.

6. Apply pipeline changes without restart:

```bash
cargo run -p cdc-ctl -- reload --daemon-url http://localhost:50051
```

7. Validate through BFF endpoints:

- `/api/cdc/health`
- `/api/cdc/metrics`
- `/api/cdc/pipelines`

## Local Multi-Sink Testing

To test the new Qdrant vector sink locally:
1. Start a local Qdrant instance (ensure it is configured with TLS if using the default https://localhost:6334 URL, or adjust the daemon code for local dev).
2. Add a Qdrant pipeline to your `pipelines.yaml`:

```yaml
- subscription_name: public.product_embeddings_sub
  sink_type: qdrant
  target_collection: products_vector_store
  id_field: product_id
  vector_field: embedding
  batch_size: 256
```

3. Ensure your upstream RisingWave/Postgres source is emitting the embedding array field.
4. Reload the daemon to pick up the new pipeline:
```bash
   cargo run -p cdc-ctl -- reload --daemon-url http://localhost:50051
```

## Notes

- The system-level contract remains in [SPECS.md](SPECS.md).
- RisingWave runnable example lives at [examples/risingwave/postgres-to-opensearch.sql](examples/risingwave/postgres-to-opensearch.sql).
