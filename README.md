# CDC Workspace Documentation Index

Use this page as the entry point for all project documentation.

## Canonical Docs (Primary)

1. [architecture_walkthrough.md](architecture_walkthrough.md)
- Architecture, control plane, security model, and enterprise operations baseline.

2. [daemon-configuration.md](daemon-configuration.md)
- Runtime configuration, environment variables, pipeline formats, and day-2 operations.

3. [image_generation_guide.md](image_generation_guide.md)
- Dockerfiles, image build and push workflows, and CI/CD image delivery guidance.

## Normative Specification

- [specs.md](specs.md)
- System contract for topology, telemetry, deployment baseline, and image versioning policy.

## Supporting Artifacts

- [k8s/cdc-starter.yaml](k8s/cdc-starter.yaml): starter Kubernetes deployment manifest.
- [examples/risingwave/postgres-to-opensearch.sql](examples/risingwave/postgres-to-opensearch.sql): runnable RisingWave CDC example DDL.

## Example Values

The repository still includes example-only placeholder values in a few starter files, including `.env` and `k8s/cdc-starter.yaml`.
These are intentional and must be replaced with environment-specific secrets, digests, and URLs before production use.

## Compatibility Docs

These files are intentionally kept as lightweight pointers for existing links:

- [development_guide.md](development_guide.md)
- [enterprise_implementation_guide.md](enterprise_implementation_guide.md)

## Suggested Reading Order

1. [README.md](README.md)
2. [architecture_walkthrough.md](architecture_walkthrough.md)
3. [daemon-configuration.md](daemon-configuration.md)
4. [image_generation_guide.md](image_generation_guide.md)
5. [specs.md](specs.md)
