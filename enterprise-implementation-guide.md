# Enterprise Implementation Guide

This file is kept for compatibility and now points to consolidated canonical docs.

## Canonical enterprise references

- Architecture and enterprise controls: [architecture-walkthrough.md](architecture-walkthrough.md)
- Runtime deployment and operational procedures: [daemon-configuration.md](daemon-configuration.md)
- Build, release, and image delivery workflow: [image-generation-guide.md](image-generation-guide.md)
- Normative specification and versioning policy: [SPECS.md](SPECS.md)

## What moved where

- Kubernetes baseline, identity and authorization model, SLO baseline, and release/rollback controls moved to [architecture-walkthrough.md](architecture-walkthrough.md).
- Environment variables, pipeline file format, cdc-ctl commands, and troubleshooting remain in [daemon-configuration.md](daemon-configuration.md).
- Dockerfile and multi-arch image workflow remains in [image-generation-guide.md](image-generation-guide.md).

## Multi-Sink Architecture Note

The Sovereign CDC system has evolved from a single-sink OpenSearch pipeline into a **multiplexed multi-sink engine**. It now natively supports routing change data streams to either **OpenSearch** (for traditional full-text search and analytics) or **Qdrant** (for high-performance vector search and AI embeddings) based on declarative pipeline configurations.

## Existing deployment artifacts

- Starter Kubernetes manifest: [k8s/cdc-starter.yaml](k8s/cdc-starter.yaml)
- RisingWave example DDL: [examples/risingwave/postgres-to-opensearch.sql](examples/risingwave/postgres-to-opensearch.sql)