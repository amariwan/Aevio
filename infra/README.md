# Infrastructure

Deployment and operations assets for a self-hosted Aevio instance. These are
optional: Aevio runs fully standalone without any of this.

- `docker/` — container build recipes (multi-stage Rust builds).
- `compose/` — Docker Compose recipes for self-hosting (PostgreSQL + server).
- `observability/` — tracing/metrics collector wiring notes.

See `docs/operations/DEPLOYMENT.md` and `docs/operations/BACKUP_RECOVERY.md`
for the operational contract these assets implement.
