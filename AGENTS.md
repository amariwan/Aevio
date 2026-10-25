# AGENTS.md

Aevio: local-first desktop productivity/time-tracking app. Rust modular monolith (Tauri 2) + React/Next.js view + optional on-device AI sidecar. macOS supported; Windows experimental (CI only); Linux unsupported.

## Non-negotiables

- Never create MVP/demo versions; build full, production-ready implementations.
- TypeScript: never use `any` to silence a linter.
- Tauri events: subscribe with `useTauriEvent` (`apps/web/src/hooks/useTauriEvent.ts`), not raw `listen`.
- User-visible reminders must use native OS notifications on macOS/Windows/Linux via the shared Rust/Tauri abstraction (notification plugin), never in-app toasts only.
- All native capability must be cross-platform by design: shared Rust/Tauri boundary + explicit per-OS impl or honest unsupported/fallback, using the `cfg_attr` platform-module pattern. Never ship macOS-only as default or emulate native behavior in React.
- Money is integer USD cents in Rust (`invoices/money.rs`). Invoice PDFs are composed in Rust (`invoices/pdf.rs`, `printpdf`) and shown via pdf.js; do not add an HTML imitation or compute totals in TS.
- Rust owns all durable state; React holds only route/transient UI and the last snapshot Rust pushed (events on change).
- UI: only shadcn/ui components (`apps/web/src/components/ui/`); always use shadcn, never build new custom components. Check shadcn first before creating anything.
## Commands

```sh
bun run dev          # dev app
bun run dev:web      # turbo dev
bun run verify       # canonical check (oxlint+oxfmt+tsc+docs metadata+cargo fmt+clippy)
bun run fix          # safe auto-fixes
just test            # cargo test --workspace + node --test scripts/*.test.mjs
```

- Run `fix` before `verify`; do not invent ad hoc oxlint/oxfmt/cargo invocations.
- Do not run `cargo build`, `cargo check`, or `bun build` speculatively; `verify` covers correctness.
- Frontend tests: `node --test scripts/*.test.mjs` (single file: `node --test scripts/breaks.test.mjs`).
- Rust tests: `cargo test --manifest-path apps/desktop/tauri/Cargo.toml` (single: append test name, e.g. `invoices::tests::renders`).
- Invoice PDF specimens: `AEVIO_INVOICE_SAMPLES=<dir> cargo test --manifest-path apps/desktop/tauri/Cargo.toml --lib invoices::tests::renders`.
- Windows clippy from Mac: `CC_x86_64_pc_windows_gnu=x86_64-w64-mingw32-gcc cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings` (in `apps/desktop/tauri/`).
- Do not add test frameworks, CI workflows, or git hooks on your own.

## Architecture

- `apps/api` = `aevio-core` Rust lib (shared by Tauri + CLI).
- `apps/cli` = `aevio` binary (talks to running app via `rpc.rs`, or reads data directly).
- `apps/desktop/tauri` = Tauri app (Rust core + Swift sidecar).
- `apps/web` = Next.js 16 static export (Turbopack), consumed by Tauri webview.
- `packages/{api-client,config,types,ui,web}` = shared TS packages.
- `apps/ios`, `apps/watch` = separate Swift apps (not in desktop verify flow).
- `crates/*` = Rust workspace crates (application, contracts, core, http, infrastructure, persistence, services; postgres-adapter).

OS-specific code uses:

```rust
#[cfg_attr(target_os = "macos", path = "capture/macos.rs")]
#[cfg_attr(windows, path = "capture/windows.rs")]
#[cfg_attr(not(any(target_os = "macos", windows)), path = "capture/unsupported.rs")]
mod platform;
```

Frontend platform conventions: `src/lib/platform.ts` as `Record<Platform, T>` — never a bare "is Mac" boolean.

## UI policy

- New UI uses shadcn/ui only (`apps/web/src/components/ui/`). Check shadcn first; no new custom components.
- Existing custom components stay as-is unless asked to migrate.
- Styling: Tailwind 4 utilities with semantic tokens in `src/styles/app.css` (`text-fg-muted`, `border-line`, `bg-surface`, ...). No raw color values.
- Appearance only from registered palettes in `apps/web/src/lib/settings.ts` + `apps/web/src/styles/themes/`. Components consume semantic tokens (`bg-canvas`, `text-fg`, ...). No raw color literals, arbitrary Tailwind color utilities, inline color styles, or parallel theming. New palette requires explicit request + frontend+Rust registration + complete light/dark token set.
- Prefer conversational page flows and open sections over stacks of decorative cards. Use cards only when grouping materially improves comprehension or interaction.
- Settings and preferences should open in a dialog from a clearly placed secondary action; do not make them a primary navigation tab unless the workflow genuinely needs a full page.
- Add charts or diagrams only when they support a concrete decision or comparison. Remove decorative, redundant, or low-signal visualizations and keep the useful values readable as text.
- Place primary actions in their local workflow and secondary actions in the relevant header/context area. Check responsive placement so controls do not compete with the main action.

## Database / persistence

- New production persistence uses SeaORM 2.0.4 via `sea-orm-sync` (rusqlite backend); no async DB rewrite without explicit decision.
- Domain modules, app services, Tauri commands, CLI handlers must not contain SQL strings, rusqlite row mappings, or SeaORM query construction; use responsibility-specific persistence adapters and map to domain/IPC models.
- Existing SQLite files, `user_version`, WAL, FKs, busy-timeout, locking, transactions, NULL semantics, indexes, timestamp units are compatibility contracts. Never run schema sync or reset data on app open.
- Historical migrations, PRAGMAs, isolated fixtures, EXPLAIN diagnostics are explicit SQL exceptions; must stay in infra/test code and be listed in `docs/DATABASE_COMPATIBILITY.md`.
- No dependency-only/unused parallel SeaORM layer; remove rusqlite impl once SeaORM replacement is active and verified.

## On-device AI sidecar

AI categorization in `apps/desktop/tauri/src/ai/`. ML calls go to Swift sidecar, built by `apps/desktop/tauri/build.rs` via `swift build`, shipped as Tauri `externalBin`. Xcode 26+ builds real sidecar; otherwise an empty placeholder is staged and app runs with rules-only fallback. Bump `protocolVersion` in `Entry.swift` and `PROTOCOL_VERSION` in `ai/sidecar.rs` together on wire-format change.

## CLI

`aevio` ships inside the app and installable standalone via `install.sh`. Talks to running app over `apps/desktop/tauri/src/rpc.rs`, or reads data via `aevio-core` when app closed. `apps/cli/COMMANDS.md` lists every command + backing Tauri command — update when either changes. Put on PATH with `aevio install-path`.

## Seeing the app

No WebDriver for macOS. Visual verify:

```sh
bash scripts/dev-screenshot.sh   # isolated instance, screenshot, teardown (macOS only)
bash scripts/app-drive.sh start  # interactive; see .agents/skills/drive-app/SKILL.md
```

Do not run `bun tauri dev` directly (port/identifier collisions). Do not write Playwright/WebDriver/E2E suites for the app window.

## Size limits

- Production files: max 500 lines (target 100–300; >400 is a warning). Functions: target max 40 lines. React presentation components: max 150 lines, no business logic in JSX.
- No artificial minification/splitting to bypass.

## Versioning / release

Every user-facing change bumps the patch version in the same PR and uses a Conventional Commit subject. Versions must match across `package.json`, `apps/cli`, `apps/api`, `tauri.conf.json`, tauri Cargo. See `RELEASING.md`. Klock (`../klock`) is the enterprise fork; changes move by hand via git-ignored port inboxes, never merge. After a shared-foundation fix merges here, record it for Klock (see `sync-klock` skill).

## CI

`.github/workflows/verify.yml` runs `bun run verify` on `macos-26` and `windows-latest` (macOS needs Xcode 26+ for sidecar), triggered by `run-ci` label. `.github/workflows/release.yml` builds signed macOS `.dmg`/`.app` on `v*` tags and publishes GitHub Release with `latest.json` updater manifest.

## Docs & knowledge

- Docs under `docs/` (`docs/README.md`, `docs/INDEX.md`); frontmatter validated by `docs/schemas/frontmatter.schema.json`.
- Source-of-truth precedence: verified source/tests → generated contracts → verified docs → ADRs → historical docs. Docs conflict with code → report, never silently reconcile.
- Retrieval: `node scripts/aevio-knowledge.mjs {search,context,feature,index,verify,eval}`.
- Task routing: `docs/agents/context-routing.md`. Rust refactoring prioritizes domain docs + dependent symbols + constraints + tests; UI styling avoids DB/AI details.
- Docs-as-Code: changes to public APIs, IPC, schema, boundaries, rules, events, config, platform support, retrieval behavior require a doc impact check. `bun run verify` includes docs metadata check.

## Graphify

Knowledge graph at `graphify-out/`. `/graphify` → use graphify skill first. For codebase questions, run `graphify query "<question>"` when `graphify-out/graph.json` exists; `graphify path "<A>" "<B>"`; `graphify explain "<concept>"`. Dirty graphify-out files are expected after hooks/incremental updates. After modifying code, run `graphify update .` (AST-only, no API cost).

<!-- gitnexus:start -->
# GitNexus — Code Intelligence

Indexed as **Aevio** (8539 symbols, 20491 relationships, 487 execution flows). Index stale? Run `node .gitnexus/run.cjs analyze --index-only`.

- MUST run impact before editing: `impact({target, direction:"upstream"})` or `node .gitnexus/run.cjs impact "symbol" --direction upstream --repo .`. Report callers/processes/risk; never substitute grep.
- MUST analyze graph changes before committing: `detect_changes({scope:"all"})` or CLI; `partial`/`truncated` is not clean. Regression review: `detect_changes({scope:"compare", base_ref:"main"})`.
- Warn on HIGH/CRITICAL risk; treat `risk: UNKNOWN` as unresolved (confirm via text search).
- Use `query`/`context`/`impact` for concepts, symbols, blast radius; text search only for empty/UNKNOWN/literals.
- Security review: `explain({target})` lists taint findings (needs `--pdg`).
- Never edit before impact; never ignore HIGH/CRITICAL; never rename via find-and-replace; never commit before change analysis.
- Resources: `gitnexus://repo/Aevio/{context,clusters,processes,process/{name}}`.
<!-- gitnexus:end -->

## Repository-aware agent system

- Primary router: `aevio-orchestrator`; specialists in `.codex/agents/`; portable workflow `.agents/skills/aevio-engineering-system/SKILL.md`.
- Route by ownership via `docs/agents/ROUTING.md`; single-agent default. Delegate only bounded independent read-only work or a clearly owned specialist slice.
- Retrieve context progressively with `scripts/aevio-knowledge.mjs`, Graphify, GitNexus (4k-token specialist target). Treat GitNexus `UNKNOWN` as unresolved.
- Every delegation states scope, exclusions, acceptance criteria, risk, stop condition, output contract. Never run conflicting writes concurrently; never expose secrets.
- Validate with `node scripts/agent-system-eval.mjs` and `docs/agents/EVALUATION.md`. Do not enable global multi-agent settings from repo files.
