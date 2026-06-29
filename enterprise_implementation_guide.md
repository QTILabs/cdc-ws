# Enterprise Implementation Guide

This file is kept for compatibility and now points to consolidated canonical docs.

## Canonical enterprise references

1. Architecture and enterprise controls: [architecture_walkthrough.md](architecture_walkthrough.md)
2. Runtime deployment and operational procedures: [daemon-configuration.md](daemon-configuration.md)
3. Build, release, and image delivery workflow: [image_generation_guide.md](image_generation_guide.md)
4. Normative specification and versioning policy: [SPECS.md](SPECS.md)

## What moved where

- Kubernetes baseline, identity and authorization model, SLO baseline, and release/rollback controls moved to [architecture_walkthrough.md](architecture_walkthrough.md).
- Environment variables, pipeline file format, cdc-ctl commands, and troubleshooting remain in [daemon-configuration.md](daemon-configuration.md).
- Dockerfile and multi-arch image workflow remains in [image_generation_guide.md](image_generation_guide.md).

## Existing deployment artifacts

- Starter Kubernetes manifest: [k8s/cdc-starter.yaml](k8s/cdc-starter.yaml)
- RisingWave example DDL: [examples/risingwave/postgres-to-opensearch.sql](examples/risingwave/postgres-to-opensearch.sql)
