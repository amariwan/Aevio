---
id: architecture.baseline
title: Aevio Baseline Matrix
category: architecture
domain: platform
status: implemented
source_paths:
  - AGENTS.md
  - Cargo.toml
  - package.json
  - apps/
  - crates/
  - packages/
related_docs:
  - docs/INDEX.md
  - docs/agents/context-routing.md
  - docs/agents/ROUTING.md
platforms:
  - macos
  - windows
  - ios
  - watchos
verification_commit: working-tree
---

# Aevio Baseline Matrix (Phase 0)

Stand: 21. Oktober 2026. Ist-Zustand vor Umsetzung des Architektur- und Migrationsmasterplans. Dient als Referenz für DRY-/Ownership-Entscheidungen und für die Verifikationsmatrix.

## 1. Apps und Crates

| Komponente | Paket/Crate | Version | Rolle | Abhängigkeiten |
|---|---|---|---|---|
| apps/api | aevio-core | 0.9.37 | Gemeinsamer Rust-Kern (SQLite + sea-orm-sync, keine HTTP) | aevio-contracts |
| apps/server | aevio-server | 0.9.35 | Axum-Sync-Server (SQLite) | aevio-contracts, aevio-http, aevio-services, aevio-core |
| apps/server-postgres | aevio-server-postgres | 0.9.35 | Axum-Sync-Server (PostgreSQL, sea-orm async) | aevio-contracts, aevio-http, aevio-services |
| apps/cli | aevio-cli | 0.9.37 | Terminal-Client (local + optional remote) | aevio-core |
| apps/desktop/tauri | aevio | 0.9.37 | Tauri-2-Desktop (standalone) | aevio-core |
| apps/web | @aevio/web | 0.9.34 | Next.js-16-Web-Client (Server-UI) | TS-Pakete |
| apps/ios | (Swift) | — | iOS-App | — |
| apps/watch | (Swift) | — | watchOS-App | — |
| crates/aevio-contracts | aevio-contracts | 0.9.35 | Versionierte Transport-/Sync-DTOs | serde |
| crates/aevio-http | aevio-http | 0.9.35 | Axum-Middleware/Handler-Bausteine | aevio-contracts, aevio-services |
| crates/aevio-services | aevio-services | 0.9.35 | Auth-/Identity-Primitive | argon2, rand, sha2 |

## 2. Lücken gegenüber Zielkarte (Masterplan §3)

- `apps/api` dient als gemeinsamer Kern (`aevio-core`); Zielkarte nennt `crates/aevio-core`. Migration optional, solange Ownership klar.
- Fehlende Crates: `aevio-application`, `aevio-persistence`, `aevio-infrastructure`. Persistence lebt in `apps/api` (SQLite) bzw. `apps/server-postgres` (PG). Infrastructure (AI, Secrets, Storage, …) lebt in `apps/api` und `apps/desktop/tauri`.
- Fehlendes Paket: `packages/features` (UI-Feature-Komposition). `packages/web` (@aevio/web-shared) existiert.
- `apps/server` (SQLite) und `apps/server-postgres` (PG) duplizieren die Route-Surface.
- Versionsdrift: Web 0.9.34, Server 0.9.35, Core/CLI/Desktop 0.9.37. Vor Release angleichen (RELEASING.md: fünf Dateien).

## 3. Server-API (identisch in beiden Servern)

| Methode | Pfad | Zweck |
|---|---|---|
| GET | /healthz | Health |
| POST | /api/v1/auth/login | Login |
| POST | /api/v1/auth/register | Registrierung |
| GET | /api/v1/workspaces | Workspace-Liste |
| POST | /api/v1/workspaces/{workspace_id}/members | Mitglied hinzufügen |
| POST | /api/v1/sync/push | Sync-Push |
| GET | /api/v1/sync/pull | Sync-Pull |

Kein `/api/v2`. OpenAPI-Vertrag in `aevio-contracts`/`aevio-http` (zu prüfen in Phase 1).

## 4. Desktop IPC

- 215 `#[tauri::command]` in `apps/desktop/tauri/src` (commands.rs + coach/*_commands.rs).
- Vertrag dokumentiert in `docs/reference/ipc.md`, `docs/reference/tauri-commands.md`.
- Frontend-Zugriff soll zentral über `apps/desktop/src/bridge/commands.ts` laufen (Masterplan §4.2).

## 5. CLI (Top-Level)

Mcp, Status, App, Track, Focus, Timers, Entries, Review, Report, Projects, Clients, Categories, Settings, Paths, InstallPath, UninstallPath, Completions (plus verschachtelte Subcommands).

## 6. Persistenz

- SQLite: `crates/aevio-core/src/bootstrap/migrations.rs` (sea-orm-sync, versioniert, WAL/Foreign Keys laut `docs/DATABASE_COMPATIBILITY.md`).
- PostgreSQL: `apps/server-postgres/src/app/core/database/migrations.rs` (sea-orm async).
- Kompatibilitätsdokument: `docs/DATABASE_COMPATIBILITY.md`, `docs/PERSISTENCE_GUIDELINES.md`.

## 7. Graph-/Index-Freshness

- GitNexus: Index vom 10/21/2026 18:50 aus Commit `1ba8d36` (== HEAD). Runner-Identity identisch. Arbeitsbaum-Abweichung nur durch untracked Graphify-Cache + drei Skripte.
- Graphify: Report vom 10/21/2026 aus Commit `1ba8d36`. 9883 Nodes, 21551 Edges, 701 Communities.

## 8. Baseline-Verifikationsbefehle

- `bun run fix` (oxfmt/oxlint/cargo fmt/clippy --fix)
- `bun run verify` (oxlint + oxfmt + tsc + cargo fmt --check + cargo clippy -D warnings)
- `cargo test --manifest-path apps/desktop/tauri/Cargo.toml`
- `cargo test --manifest-path apps/cli/Cargo.toml`
- `cargo test --manifest-path apps/server-postgres/Cargo.toml`
- `node --test scripts/*.test.mjs`
- `node scripts/aevio-knowledge.mjs verify`

Diese Befehle wurden in dieser Phase nicht ausgeführt (keine spekulativen Builds); Ergebnisse sind in den folgenden Phasen zu erfassen.

## 9. Nicht verhandelbare Constraints (aus AGENTS.md)

- Local-first: Desktop bootet ohne Account/Server/Internet.
- Keine Hintergrund-Uploads persönlicher Rohaktivität/Fensterdaten/Coach-Daten/Dateien ohne explizite Freigabe.
- Native OS-Notifications für User-Reminder auf macOS/Windows/Linux (kein Toast-only).
- Alle nativen Funktionen cross-platform über cfg_attr-Pattern; nie macOS-only als Default.
- `useTauriEvent` statt rohem `listen`.
- TypeScript: kein `any`.
- Neue UI nur shadcn/ui; Styling über semantische Tokens in `src/styles/app.css`.
- Nur registrierte Paletten aus `src/lib/settings.ts` + `src/styles/themes/`.
- Money in Integer-Cents (Rust `invoices/money.rs`); Invoice-PDFs in Rust (`invoices/pdf.rs`).
- Rust-DB: SeaORM 2.0.4 via sea-orm-sync mit rusqlite; keine SQL-Strings in Domain/Services/Commands.
- Dateigröße ≤ 500 LOC (Ziel 100–300), Funktionen ≤ 40 Zeilen.
- Keine Tests/CI/Hooks eigenmächtig hinzufügen.
- Keine automatischen Commits/PRs/Pushes/Deployments.
path: docs/architecture/baseline.md