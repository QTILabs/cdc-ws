# CDC Workspace Development Guide

This file is now the quickstart and index. Canonical operational content is consolidated into three primary docs:

1. [architecture_walkthrough.md](architecture_walkthrough.md): architecture, control plane, security model, and enterprise operating baseline
2. [daemon-configuration.md](daemon-configuration.md): runtime configuration, environment variables, cdc-ctl operations, and troubleshooting
3. [image_generation_guide.md](image_generation_guide.md): Dockerfiles, image build/push workflow, and CI/CD image practices

## Quickstart

1. Configure runtime values in `.env` and pipeline definitions in `pipelines.yaml`.
2. Start daemon:

```bash
cargo run -p cdc-ctl -- start --foreground
```

3. Start BFF:

```bash
cargo run -p cdc-bff
```

4. Start web console:

```bash
cd cdc-web-console
pnpm dev
```

5. Apply pipeline changes without restart:

```bash
cargo run -p cdc-ctl -- reload --daemon-url http://localhost:50051
```

6. Validate through BFF endpoints:

- `/api/cdc/health`
- `/api/cdc/metrics`
- `/api/cdc/pipelines`

## Notes

- The system-level contract remains in [SPECS.md](SPECS.md).
- RisingWave runnable example lives at [examples/risingwave/postgres-to-opensearch.sql](examples/risingwave/postgres-to-opensearch.sql).
