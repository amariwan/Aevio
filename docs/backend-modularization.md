# Aevio backend modularization map

This map records the ownership boundaries used by `apps/api` while preserving
the compatibility facades consumed by the desktop app and CLI.

## Boundaries

| Domain | Public entry point | Responsibilities |
| --- | --- | --- |
| `entries` | `entries::models`, `entries::builder`, `entries::query` | Durable entry/catalog types, segment-to-entry construction, filters and row mapping |
| `activity` | `activity::ActivityStore` | Capture lifecycle, catalog mutations, entry lifecycle, undo/redo and activity snapshots |
| `ai` | `ai::{arbiter, calibration, knn, rules, store}` | Suggestions, learning, model artifacts and queue persistence |
| `settings` | `settings::{models, validation, storage, tracking, breaks}` | JSON-compatible preferences, normalization, atomic persistence and schedules |
| `bootstrap` | `bootstrap::{migrations, paths, state}` | Database startup, schema migration, filesystem paths and shared application state |
| `projects` | `projects::service` | Project hints, discovery, import and project statistics |
| `reports` | `reports::query` | Filtered reads, rollups and exports |
| `ipc` | `ipc::{protocol, rpc}` | Local transport, wire contracts and request dispatch |

## Compatibility

The root exports `EntryFilter`, `query_entries`, `models`, `entry_builder`,
`protocol`, `rpc`, `migrations`, `paths`, and `state` remain available. New
code should import from the owning domain instead of the root facade.

## Refactoring rules

- Domain code may depend on shared models and narrow infrastructure seams, not
  on desktop/Tauri code.
- Settings normalization is the single write boundary; storage never mutates
  the in-memory snapshot before an atomic write succeeds.
- SQL row mapping and filter construction live with entries, while report
  aggregation remains in `reports`.
- The large activity and migration stores remain intentionally transactional;
  their next splits must preserve private-state access and migration ordering.
