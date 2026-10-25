# Aevio – Principal Engineer Architecture Review & Refactoring Strategy

> Status: Planungsdokument (Revision 2, kritisch validiert). Kein Code geändert. Keine Branches/Commits/PRs.
> Repository: `amariwan/Aevio` @ `main` (Version 0.9.17) — Produktname-Ziel: **Aevio**; Code-Identifier (`aevio-core`, `aevio_lib`, `com.offlinestudios.aevio`) bleiben bis zu einem separaten Rename-PR unverändert.
> Erstellt: 2026-10-19

---

## 1. Executive Summary

Aevio (Repo Aevio) ist ein Tauri-2-Desktop-Modulmonolith mit drei Binärtargets (App, CLI `aevio`, Swift-Sidecar) und einer React-19-View. Die Datenhaltung ist sauber zentralisiert (`aevio-core` Crate + SQLite), aber die **Modulgrenzen sind überwiegend implizit**: ein 33-kLOC-Core-Crate mit flachem Modulbaum, ein 1216-Zeilen-Command-Handler, ein 659-Zeilen-Frontend-IPC-Modul, kein Feature-Isolation, keine Public-API-Barrels, Event-Strings in zwei Sprachen dupliziert.

**Validierung der 16 Vorbefunde (§2):** 12 bestätigte Architekturprobleme/technische Schuld, 2 bewusste Architekturentscheidungen, 2 optionale Verbesserungen. Kein Vorbefund war unbegründet; zwei wurden herabgestuft (F6 `AppState`-Mutex-Bündel ist eine bewusste, vertretbare Entscheidung; F13 verteilte AI-Grenze ist bewusst und funktional).

**Empfohlene Zielarchitektur:** Modular Monolith + Feature-Slices + Hexagonal Ports **nur an drei echten Integrationsgrenzen** (SQLite-Repository, Sidecar-Client, Capture-Platform). Keine Microservices, kein Framework-Wechsel, keine neuen Crates ohne nachgewiesenen Bedarf, keine neuen State-Management-Libraries.

**Erster Implementierungs-PR:** Frontend-Barrel-Files (`index.ts`) in `components/`, `hooks/`, `lib/` ohne Verhaltensänderung — risikoarm, etabliert eine wiederverwendbare Konvention (kontrollierte Public APIs).

---

## 2. Validierung der 16 Vorbefunde

Klassifizierung: **BP** = bestätigtes Architekturproblem, **TS** = technische Schuld, **BE** = bewusste Architekturentscheidung, **OV** = optionale Verbesserung, **UB** = nicht ausreichend belegt.

| # | Vorbefund | Klasse | Evidenz (Datei:Zeile) | Bewertung |
|---|---|---|---|---|
| F1 | `activity.rs` God-Module (3932 LOC) | **BP** | `src-tauri/core/src/activity.rs` 3932 Zeilen; enthält Store, Segment-Folding, Queries, Deletions, Undo/Redo (`history.rs` 997) | Bestätigt. Eine Datei mischt Persistenz, Domänenregeln (`ensure_no_approved_overlap`, `LOCKED`) und Query-Logik. |
| F2 | Domain-Logik im Crate-Root | **BP** | `src-tauri/core/src/lib.rs` definiert `EntryFilter`, `time_entry_from_row`, `query_entries` (SQL); `rpc.rs:26` importiert `EntryFilter` aus Crate-Root | Bestätigt. Filter-Semantik (Domain) und SQL (Infra) vermischt. |
| F3 | `commands.rs` zentraler Kopplungspunkt | **TS** | `src-tauri/src/commands.rs:1-40` importiert 16 Module | Bestätigt, aber: dünne Transportadapter sind legitim; Problem ist fehlende Use-Case-Schicht, nicht die Datei selbst. |
| F4 | `api.ts`/`types.ts` Flaschenhälse | **BP** | `src/lib/api.ts` 659 Zeilen, `src/lib/types.ts` 734; Import-Häufigkeit `../lib/api` 33×, `../lib/types` 26× | Bestätigt. Jede Feature-Änderung berührt beide Dateien. |
| F5 | Geschäftslogik in UI-Komponenten | **TS** | `src/components/EntryReviewPanel.tsx` 1181, `src/pages/Settings.tsx` 1653, `src/pages/TimeEntries.tsx` 1498 | Bestätigt. Nicht alle 1653 Zeilen sind Logik, aber State/IPC-Hooks fehlen. |
| F6 | `AppState` Mutex-Bündel | **BE** | `src-tauri/core/src/state.rs:19-34` | **Herabgestuft.** Bewusste Entscheidung für eine Desktop-App mit Single-Writer; `activity_reader` "by convention" ist jedoch echte TS (keine Typ-Grenze). Teilproblem bleibt. |
| F7 | Windows-Capture `RefCell`-Statics | **TS** | `src-tauri/src/capture/windows.rs:77-78` | Bestätigt. Globaler mutable State, nicht testbar. |
| F8 | Event-Strings dupliziert | **BP** | `src-tauri/src/lib.rs` `EVENT_*` vs. `src/lib/api.ts:1-24` eigene Konstanten | Bestätigt. Keine Single Source of Truth. |
| F9 | `models.rs` Typ-Dump | **TS** | `src-tauri/core/src/models.rs` 440 Zeilen, alle Domänen | Bestätigt, aber moderate Schwere (440 LOC ist nicht extrem). |
| F10 | Appearance-Init-Script in Settings | **TS** | `src-tauri/src/lib.rs` `settings::appearance_init_script` | Bestätigt. Präsentationslogik in Datenmodul. |
| F11 | Invoices PDF/Domain gemischt | **TS** | `src-tauri/src/invoices/{mod.rs 931, pdf.rs 676, profile.rs 260}` | Bestätigt. |
| F12 | Breaks Engine/Surface gemischt | **TS** | `src-tauri/src/breaks/{engine.rs 1697, surface.rs 321}` | Bestätigt. |
| F13 | AI über zwei Crates verteilt | **BE** | `core/src/ai/*` (Daten/Regeln) + `src/ai/*` (Runtime); `src/ai/mod.rs` `pub use aevio_core::ai::*` | **Bewusst und funktional.** Die Grenze (Core=Daten, App=Runtime) ist sinnvoll; fehlt nur eine explizite Port-Definition (Trait). |
| F14 | Projects Regex+DB gemischt | **TS** | `src-tauri/core/src/projects.rs:47,336,382,614` | Bestätigt. Pure Discovery nicht isoliert. |
| F15 | `useSettings.tsx` Context+Provider | **OV** | `src/hooks/useSettings.tsx` 392 Zeilen | Optionale Verbesserung. React-übliches Muster; Problem ist Größe, nicht Struktur. |
| F16 | Keine Barrel/Public-API-Dateien | **BP** | `find src -name index.ts` → leer | Bestätigt. Keine kontrollierten Import-Grenzen. |

**Zusammenfassung:** 8 BP, 6 TS, 2 BE, 1 OV (F15), 0 UB. Kein Vorbefund war unbegründet.

---

## 3. Ist-Architektur und Dependency-Graph

### 3.1 Binär-/Crate-Topologie

```
┌──────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  App (tauri) │────▶│ aevio-core    │◀────│ aevio CLI        │
│ src-tauri/   │     │ (shared crate)   │     │ src-tauri/cli/  │
└──────┬───────┘     └────────┬─────────┘     └─────────────────┘
       │ stdio/JSON           │
       ▼                      │
┌──────────────┐              │
│ Swift sidecar│              │
│ aevio-ml  │              │
└──────────────┘              │
       ▲                      │
       └──── Tauri events ────┘?  (nein: Sidecar nur via Rust)
```

App und CLI teilen `aevio-core` (Pfad-Dep). Sidecar ist externes Binary (Tauri `externalBin`), JSON-Lines über stdio. Frontend spricht nur über Tauri IPC.

### 3.2 Rust-Core interne Abhängigkeiten (aus `use crate::`)

```
activity ──▶ capture, entry_builder, models, timers
ai/arbiter ──▶ models, ai/rules, ai (COLD_START_CAP)
ai/rules ──▶ activity (ActivitySegment, KIND_BREAK)
ai/store ──▶ models
projects ──▶ activity, ai/rules, models, err
reports ──▶ models, err, EntryFilter, activity::ActivityStore, NONE
settings ──▶ timers
state ──▶ activity, paths, protocol, settings, timers
timers ──▶ activity (DB_FILE)
rpc ──▶ models, protocol, reports, settings, state, timers, EntryFilter, NONE
protocol ──▶ models, EntryFilter
paths ──▶ APP_IDENTIFIER (lib.rs)
```

Zyklen: keine harten Zyklen auf Modul-Ebene (Rust verhindert das). **Aber:** `projects` → `activity` → `entry_builder` → (models) und `ai/rules` → `activity` erzeugen eine dichte Kopplung: fast alle Domänen hängen an `activity`. Das ist der zentrale Kopplungspunkt, nicht `commands.rs`.

### 3.3 Frontend-Import-Graph (Top)

- `src/lib/api.ts` (659) und `src/lib/types.ts` (734) sind die meistimportierten Module (46 bzw. 43 Import-Sites).
- `src/components/Page.tsx` 10×, `src/components/Tooltip.tsx` 11× — generische UI, aber ohne Geschäftslogik (akzeptabel).
- Keine `index.ts`-Barrels → Imports gehen in konkrete Dateien; Refactoring bricht viele Call-Sites.

### 3.4 Globale Zustände

- Rust: `AppState` (Mutex-Bündel), `AiRuntime`, `PulseState`, `SurfaceState`, Windows-`RefCell`-Statics, `static FONTS: OnceCell` (invoices/pdf.rs:90), LazyLock-Regexes (projects.rs).
- Frontend: `SettingsProvider` (Context), `localStorage` (Sidebar), mehrere `useState` in `App.tsx`.

---

## 4. Verbindliche Zielarchitektur

### 4.1 Entscheidung: Modular Monolith + Feature Slices + Hexagonal Ports (nur 3)

**Begründung pro Ansatz:**

| Ansatz | Bewertung | Entscheidung |
|---|---|---|
| Modular Monolith | Passt: eine Deployable-Einheit, App+CLI+Sidecar | ✅ Basis |
| Feature-Based Architecture | Passt für Frontend und Rust-Module | ✅ |
| Vertical Slice Architecture | Gut für durchgängige Features (z.B. Invoice: UI→Command→Use-Case→DB) | ✅ als Organisationsprinzip für PRs |
| Hexagonal Architecture | Nur an echten Integrationsgrenzen sinnvoll | ✅ 3 Ports (§4.6) |
| DDD light | Bounded Contexts = Domänen; Aggregates dort, wo Invarianten existieren | ✅ |
| Rust Workspace mehrerer Crates | Vorläufig nicht; siehe §4.2 | ❌ jetzt |

**Keine Microservices** (explizit ausgeschlossen). **Keine neuen State-Management-Libraries** (React 19 + Context reicht).

### 4.2 Wann eine zusätzliche Crate-Grenze sinnvoll ist

**Sinnvoll, wenn:**
- Ein zweiter Consumer die Domain **ohne Infra** nutzen will (heute: nein — CLI nutzt Core komplett).
- Compile-Zeit > 2 Min und Incremental-Builds leiden (heute: unbekannt, nicht gemessen → kein Grund).
- Eine Domain stabil und unabhängig ist und als Library publiziert werden soll (heute: nein).

**Nicht sinnvoll jetzt**, weil:
- `state::lock_stores`-Semantik (App hält Lock, CLI nimmt ihn) erfordert, dass Store + Lock + Migration in einem Crate leben.
- Eine Aufspaltung in `aevio-domain` + `aevio-infra` würde die Migration-Tests und den `SCHEMA_VERSION`-Check aufbrechen, ohne Consumer-Nutzen.
- **Regel:** Neue Crate nur bei nachgewiesenem Bedarf (zweiter Consumer oder Messung). Interne Modulgrenzen mit `pub`/`pub(crate)` reichen.

### 4.3 Bounded Contexts (DDD light)

| Context | Domänenobjekte | Invarianten | Crate/Modul |
|---|---|---|---|
| Activity/Entries | `ActivitySegment`, `TimeEntry`, `EntryFilter` | Approved-Lock, Overlap-Check, Segment-Folding | `core::domain::activity`, `core::domain::entries` |
| Projects/Clients | `Project`, `Client`, `ProjectRule`, Discovery | Budget-Parsing, Domain-Klassifikation | `core::domain::projects` |
| Invoices | `Invoice`, `InvoiceDraftInput`, Status-Maschine | Billable-Entries, Finalize/void | `core::domain::invoices` + `app::invoices` (PDF) |
| Timers | `Timer`, `TimerSegment` | — | `core::domain::timers` |
| Settings | `Settings`, Appearance | — | `core::domain::settings` |
| AI | Arbiter, Calibration, Rules, KNN-Store | Confidence-Cap, Cold-Start | `core::domain::ai` + `app::ai` (Runtime) |
| Breaks | `Engine`, `BreakState`, `BreakRecord` | Scheduling | `app::breaks` |
| Capture | `WindowSample`, Platform-Sampling | — | `app::capture` (cfg_attr) |
| Reports | Rollup, Export | — | `core::domain::reports` |

### 4.4 Abhängigkeitsrichtung (verbindlich)

```
Frontend features/* ──▶ shared/ipc/* ──▶ (Tauri IPC) ──▶ app::commands/* ──▶ core::application/* ──▶ core::domain/* 
                                                                                      │
                                                                                      ▼
                                                                              core::infrastructure/* (SQLite, FS, RPC)
app::ai (Runtime) ──▶ core::domain::ai + app::ai::port (Sidecar Trait)
app::capture ──▶ core::domain (nur WindowSample) + platform impls
```

**Regeln:**
- `core::domain/*` darf nichts aus `infrastructure`, `application`, `app`, `tauri` importieren.
- `core::application/*` darf `domain` + `infrastructure` (Repository-Traits) nutzen, aber keine Tauri-Typen.
- `app::commands/*` sind dünne Adapter: parse → use case → emit.
- Frontend: `features/*` importieren nur `shared/*` und eigene Feature-Dateien; Cross-Feature nur über `index.ts` (Public API).
- `shared/*` importiert keine Features.

### 4.5 Öffentliche Modul-APIs

- Rust: Jedes Modul deklariert `pub` nur, was wirklich extern nötig ist; Interna `pub(crate)`. Crate-Root (`lib.rs`) re-exportiert eine stabile, dokumentierte API (`pub use domain::…`, `pub use application::…`).
- Frontend: Jedes Feature hat `index.ts` mit expliziter Export-Liste; Deep-Imports (`features/x/components/Y`) sind verboten außerhalb des Features.
- CLI: `aevio-core` Public API ist der Vertrag; Breaking Changes erfordern `SCHEMA_VERSION`-Bump oder Protokoll-Version.

### 4.6 Hexagonal Ports (genau drei)

1. **`EntryRepository` / `ActivityRepository`** (Trait in `core::domain`, Impl in `core::infrastructure::sqlite`) — ermöglicht In-Memory-Tests.
2. **`AiSidecarClient`** (Trait in `app::ai::port`, Impl JSON-Lines in `app::ai::sidecar`) — ermöglicht Mock für Arbiter/Calibration-Tests.
3. **`CapturePlatform`** (Trait in `app::capture`, Impls in `macos.rs`/`windows.rs`/`unsupported.rs`) — macht Sampling testbar.

Keine weiteren Ports (Settings, Timers, etc.) — YAGNI; dort reicht das bestehende Muster.

### 4.7 Fehlerbehandlung

- Core: `thiserror`-Enum `DomainError` (derzeit `String`). Migration: schrittweise; neue Use-Cases nutzen `DomainError`, alte bleiben `String` bis zum Split.
- RPC: `ApiError { code, message }` bleibt (Protokoll-Vertrag).
- Tauri-Commands: `Result<T, String>` bleibt (Tauri-IPC-Konvention).
- Keine Panics in Library-Code; `unwrap` nur in Tests.

### 4.8 Sync/Async-Datenfluss

- **Sync:** Use-Case-Funktionen sind sync (`fn foo(conn: &Connection, …) -> Result`). SQLite-Aufrufe blockieren kurz; Tauri-Commands laufen im async-Runtime, aber die Store-Locks sind sync (`Mutex`). Das ist die bestehende Konvention (A6-Entscheidung im activity.rs-Doc) — beibehalten.
- **Async:** Worker-Thread (AI), Sampler-Threads (Capture/Energy), Retention-Sweeper laufen in `std::thread`. Events werden über `app.emit` gepusht. Keine `tokio`-Tasks im Core.
- **Regel:** Keine neuen async-Abstraktionen; bestehende Thread-Modell beibehalten.

---

## 5. Rust-Core Zielstruktur (intern)

```
src-tauri/core/src/
├── lib.rs                          # nur Re-Exports + Metadaten, keine Logik
├── domain/
│   ├── activity/
│   │   ├── mod.rs                  # ActivitySegment, Segment-Folding-Regeln
│   │   ├── approved_lock.rs        # LOCKED, Overlap-Checks
│   │   └── entry_builder.rs        # pure Folding (SegmentInput → BuiltTimeEntry)
│   ├── entries/
│   │   ├── mod.rs                  # TimeEntry
│   │   └── filter.rs               # EntryFilter + Semantik (NONE, Search-Terms)
│   ├── projects/
│   │   ├── mod.rs                  # Project, Client, ProjectRule
│   │   └── discovery.rs            # Regex-Klassifikation (pure, testbar)
│   ├── invoices/
│   ├── timers/
│   ├── settings/
│   ├── ai/
│   │   ├── arbiter.rs
│   │   ├── calibration.rs
│   │   ├── rules.rs
│   │   ├── knn.rs
│   │   └── store.rs                # KNN-Store-Trait + Impl
│   ├── reports/
│   └── breaks/
├── application/
│   ├── activity_usecases.rs
│   ├── entry_usecases.rs
│   ├── project_usecases.rs
│   ├── invoice_usecases.rs
│   └── ...
├── infrastructure/
│   ├── sqlite/
│   │   ├── activity_repo.rs
│   │   ├── entry_query.rs          # EntryFilter → SQL (aus lib.rs)
│   │   └── migrations.rs           # (umgezogen aus core/src/migrations.rs)
│   ├── fs/
│   │   └── paths.rs
│   └── rpc/
│       ├── protocol.rs
│       ├── ipc.rs
│       └── server.rs               # rpc.rs
├── platform/                       # (nur Traits, Impl in app/capture)
└── models.rs                       # nur IPC-DTOs (oder in domain/*/dto.rs)
```

**Migrationsregel:** Crate-Root (`lib.rs`) re-exportiert alle bisherigen Pfade (`pub use domain::entries::EntryFilter`, `pub use infrastructure::sqlite::entry_query::query_entries`), sodass `aevio-core::EntryFilter` und `aevio-core::query_entries` weiterhin kompilieren. App und CLI brechen nicht.

---

## 6. Tauri Application Layer Zielstruktur

```
src-tauri/src/
├── lib.rs                          # Tauri-Builder, Wiring, Event-Konstanten (Re-Export core)
├── commands/
│   ├── mod.rs
│   ├── activity.rs
│   ├── entries.rs
│   ├── projects.rs
│   ├── clients.rs
│   ├── timers.rs
│   ├── invoices.rs
│   ├── settings.rs
│   ├── ai.rs
│   ├── apps.rs
│   ├── breaks.rs
│   ├── reports.rs
│   └── system.rs                   # permissions, updater, pulse, tray
├── runtime/
│   ├── state.rs                    # AppState (move aus core? bleibt app-seitig)
│   ├── sampler.rs                  # activity::spawn_sampler wrapper
│   ├── tray.rs
│   ├── pulse.rs
│   └── updater.rs
├── ai/
│   ├── mod.rs                      # AiRuntime + Status
│   ├── worker.rs
│   ├── power.rs
│   ├── metrics.rs
│   ├── features.rs
│   └── port.rs                     # AiSidecarClient Trait
│   └── sidecar.rs                  # Impl
├── breaks/
│   ├── mod.rs                      # Engine
│   ├── stopwatch.rs
│   ├── store.rs
│   └── surface.rs                  # UI-Placement (app-seitig, bleibt)
├── capture/, energy/, login_item/, permissions/   # cfg_attr (unverändert)
├── invoices/
│   ├── mod.rs                      # Lifecycle-Commands-Helper
│   ├── pdf.rs                      # printpdf (Infra)
│   ├── profile.rs
│   └── money.rs
└── rpc.rs                          # RPC-Server (app-seitig)
```

**Commands bleiben dünne Transportadapter** (≤ ~30 Zeilen): parse Args → rufe `core::application::*` → emit Event → return. Geschäftsregeln in Core.

**Kompatibilität:** Command-Namen, Payloads (serde camelCase), Event-Namen bleiben identisch. Nur interne Dateiorganisation ändert sich.

---

## 7. React-Frontend Zielstruktur

```
src/
├── app/
│   ├── App.tsx
│   ├── routes.ts                   # Route-Union + History (aus lib/types.ts)
│   ├── providers/
│   │   └── SettingsProvider.tsx    # aus hooks/useSettings.tsx
│   └── layout/
│       ├── Sidebar.tsx
│       ├── TopBar.tsx
│       └── Page.tsx
├── features/
│   ├── activity/                   # Calendar, Day/Week/Month, EntryBlock
│   │   ├── api.ts
│   │   ├── types.ts
│   │   ├── hooks/
│   │   ├── components/
│   │   └── index.ts
│   ├── entries/                    # TimeEntries, filters, rollup, export
│   ├── projects/
│   ├── clients/
│   ├── timers/
│   ├── invoices/
│   ├── settings/
│   ├── ai/
│   ├── apps/
│   ├── breaks/
│   ├── reports/                    # Timesheets grid, charts
│   └── pulse/                      # Pulse panel + Reminder window
├── shared/
│   ├── ui/                         # ConfirmDialog, Picker, Sheet, Tooltip, ...
│   ├── hooks/                      # useTauriEvent, useMediaQuery, ...
│   ├── ipc/
│   │   ├── commands.ts             # typisierte invoke-Wrapper
│   │   ├── events.ts               # Event-Namen + Payload-Typen
│   │   └── types.ts                # DTOs 1:1 zu Rust-serde
│   └── lib/                        # dates, format, platform, ...
└── main.tsx
```

**Regeln (verbindlich):**
1. Fachliche Komponenten bleiben in ihrem Feature.
2. `shared/` enthält keine fachliche Geschäftslogik.
3. Features kommunizieren über `index.ts` (Public API).
4. Keine Cross-Feature-Deep-Imports (`features/x/components/Y` von außen verboten).
5. Keine unnötigen Barrel-Dateien — `index.ts` nur dort, wo eine Public API sinnvoll ist (jedes Feature hat genau einen).
6. Keine neue State-Management-Library.
7. `useTauriEvent` bleibt die einzige Event-Subscription (AGENTS.md).

---

## 8. Typisierte IPC-Verträge

### 8.1 Problem

- Commands, Events und DTOs sind in Rust (serde) und TS (manuell) definiert. Keine Codegen. Event-Strings dupliziert (F8).
- Risiko: Tippfehler zur Laufzeit; keine Compile-Zeit-Garantie.

### 8.2 Bewertete Lösungen

| Lösung | Aufwand | Nutzen | Empfehlung |
|---|---|---|---|
| `ts-rs` / `specta` Codegen aus Rust | Mittel (Build-Setup, Macro-Annotate) | Hoch (typsichere DTOs) | **Später (PR nach IPC-Registry)** |
| Gemeinsame Event-Registry (manuell, sync-geprüft) | Niedrig | Mittel | **Sofort (PR-E)** |
| JSON-Schema aus Rust + TS-Types | Hoch | Mittel | ❌ Overengineering |
| Protobuf/gRPC | Sehr hoch | Niedrig (Desktop-IPC) | ❌ |

### 8.3 Empfohlene Lösung (minimaler Build-Aufwand)

**Phase 1 (sofort):** `shared/ipc/{commands.ts, events.ts, types.ts}` als manuelle, aber **zentralisierte** Registry. Event-Strings werden aus einer einzigen Rust-Quelle (`pub const EVENT_*` in `app::lib`) per Build-Script oder (pragmatischer) per manueller Sync mit CI-Check abgeglichen (grep-Script vergleicht String-Literale).

**Phase 2 (optional, später):** `ts-rs` für DTOs, sobald die IPC-Registry stabil ist und ein zweiter Consumer (z.B. Web-Debug-UI) entsteht. Nicht vorher — Codegen-Setup ist Aufwand ohne unmittelbaren Nutzen bei einem einzelnen Frontend.

**Sidecar/RPC-Protokoll:** `protocol.rs` hat `PROTOCOL_VERSION` und `Entry.swift` hat `protocolVersion`. Diese müssen **zusammen** gebumpt werden (AGENTS.md). Empfehlung: CI-Check, der beide Versionen vergleicht (grep auf `PROTOCOL_VERSION` und `protocolVersion`).

---

## 9. Architekturregeln (überprüfbar)

### 9.1 Rust-Modulgrenzen

- `aevio-core` darf nicht `tauri`/`tauri-*` dependen → `cargo deny` (bans) Regel.
- `core::domain/*` darf keine `rusqlite`, `std::fs`, `interprocess` importieren → Clippy/`cargo deny` oder grep-basiertes CI-Script.
- `core::application/*` darf keine Tauri-Typen importieren.
- Datei-Größe: Warnung > 1000 LOC (optional, P3).

### 9.2 TypeScript-Importgrenzen

- `depcruise` oder `eslint-plugin-boundaries`: Features dürfen nicht in andere Features deep-importen; `shared/*` importiert keine Features.
- Biome `files.includes` (bereits) für Formatierung/Lint.

### 9.3 Öffentliche APIs

- Rust: `pub` nur an Modulgrenzen; Interna `pub(crate)`.
- Frontend: `index.ts` pro Feature; keine Deep-Imports von außen.

### 9.4 Dependency-Zyklen

- Rust: Compiler verhindert Zyklen; aber Architektur-Zyklen (z.B. `projects` ↔ `activity`) werden durch Layering (Domain darf nicht Application importieren) verhindert → Code-Review + CI-Script.

### 9.5 Typensicherheit

- TS `strict: true` (bereits). Kein `any` (AGENTS.md). Biome recommended (bereits).

### 9.6 Fehlerbehandlung

- Core: `DomainError` (thiserror) für neue Use-Cases; alte `String`-Pfade bleiben bis Split.
- Keine `unwrap` in Library-Code.

### 9.7 SQLite-Transaktionen

- `BUSY_TIMEOUT` (5s) beibehalten. Reader/Writer als getypte Handles (nicht `Mutex<Connection>` by convention).
- Migrations additiv; `SCHEMA_VERSION`-Bump nur bei Breaking-Änderung.

### 9.8 Plattformabhängigkeiten

- `cfg_attr`-Pattern (AGENTS.md) beibehalten. Windows-Clippy-Target in CI.

### 9.9 Regressionstests

- Rust: `cargo test`. Frontend: `node --test scripts/*.test.mjs`.
- DB-Smoke: `state.rs`-Tests (Schema-Version, NO_DATA).

### 9.10 CLI-Kompatibilität

- `COMMANDS.md` aktuell halten (PR-Checklist). RPC-Protokoll nicht brechen.

### 9.11 Werkzeuge (bewertet)

| Werkzeug | Zweck | Einführung |
|---|---|---|
| `cargo deny` | Dependency-Bans (tauri in core) | PR-N (nach Core-Split) |
| `depcruise` | TS-Importgrenzen | PR-N |
| `thiserror` | DomainError | PR-G (Core-Split) |
| `ts-rs` | Codegen DTOs | Später (optional) |
| `cargo clippy` | Lints | Bereits (verify) |
| Biome | Format/Lint | Bereits (verify) |

---

## 10. Optimierter PR-Plan (priorisiert)

Kein Big-Bang. Jeder PR einzeln reviewbar/revertierbar. Keine Struktur- und Verhaltensänderung in einem PR.

### PR-1: Frontend-Barrels (risikoarm, etabliert Konvention)
- **Ziel:** Kontrollierte Public APIs in `components/`, `hooks/`, `lib/`.
- **Dateien:** neue `src/components/index.ts`, `src/hooks/index.ts`, `src/lib/index.ts`; Imports umstellen.
- **Änderungen:** Nur Re-Exports + Import-Pfad-Updates (mechanisch).
- **Risiko:** Niedrig.
- **Akzeptanz:** `bun run verify` grün; keine Verhaltensänderung.
- **Tests:** Biome + tsc.
- **Rollback:** Revert.

### PR-2: Frontend Feature-Ordner (Pilot `timers`)
- **Ziel:** Feature-Slice-Konvention etablieren.
- **Dateien:** `src/features/timers/{api.ts,types.ts,hooks/,components/,index.ts}`; `Timers.tsx` konsumiert nur Public API.
- **Risiko:** Niedrig-Mittel.
- **Akzeptanz:** `verify` grün; Smoke-Test Timers-View.
- **Rollback:** Revert.

### PR-3..N: Verbleibende Feature-Ordner (entries, projects, clients, invoices, settings, ai, apps, breaks, reports, activity)
- Ein PR pro Domäne oder 2–3 gebündelt. Risiko: Mittel.

### PR (Core): Core interne Reorganisation (Pilot `projects`)
- **Ziel:** `domain/projects/discovery.rs` extrahieren; Re-Exports halten API stabil.
- **Dateien:** `src-tauri/core/src/projects.rs` → `domain/projects/{mod.rs,discovery.rs}` + `infrastructure/sqlite/project_repo.rs`; `lib.rs` Re-Exports.
- **Risiko:** Niedrig.
- **Akzeptanz:** `cargo test` + `verify` grün; CLI-Tests grün.

### PR (Core): `EntryFilter`/`query_entries` verschieben (F2)
- **Ziel:** Filter-Semantik (Domain) vs. SQL (Infra) trennen.
- **Dateien:** `core/src/lib.rs` → `domain/entries/filter.rs` + `infrastructure/sqlite/entry_query.rs`.
- **Risiko:** Mittel (SQL).
- **Akzeptanz:** `cargo test` + `verify`; bestehende DBs öffnen sich.

### PR (Core): `activity.rs` aufteilen (F1) — in 2–3 Sub-PRs
- **Ziel:** Store/Segment-Folding, History/Undo, Deletions trennen.
- **Dateien:** `core/src/activity.rs` → `domain/activity/*` + `application/activity_usecases.rs` + `infrastructure/sqlite/activity_repo.rs`.
- **Risiko:** Hoch.
- **Akzeptanz:** `cargo test` + `verify` + manueller Smoke (Tracking, Review, Undo).

### PR (App): `commands.rs` aufteilen (F3)
- **Ziel:** Ein Modul pro Domäne.
- **Dateien:** `src-tauri/src/commands.rs` → `src-tauri/src/commands/*.rs`.
- **Risiko:** Niedrig (mechanisch).
- **Akzeptanz:** `verify` grün; Command-Namen/Payloads unverändert.

### PR (Core): `models.rs` aufteilen (F9)
- **Ziel:** Domain-Modelle vs. IPC-DTOs.
- **Dateien:** `core/src/models.rs` → `domain/<ctx>/model.rs` + `ipc/dto.rs`; Re-Exports.
- **Risiko:** Mittel.
- **Akzeptanz:** `cargo test` + `verify`.

### PR (App): Sidecar-Port-Trait (F13)
- **Ziel:** `AiSidecarClient` Trait + Mock-Tests.
- **Dateien:** `src-tauri/src/ai/port.rs` (neu), `sidecar.rs` Impl, `worker.rs` nutzt Trait.
- **Risiko:** Mittel.
- **Akzeptanz:** `cargo test` (Arbiter/Calibration mit Mock).

### PR (App): Capture Windows Global State → State (F7)
- **Ziel:** `RefCell`-Statics in `CaptureState`.
- **Dateien:** `src-tauri/src/capture/windows.rs`, `runtime/state.rs`.
- **Risiko:** Niedrig-Mittel.
- **Akzeptanz:** Windows-Clippy-Target grün.

### PR (App): Settings Appearance-Script aus Core (F10)
- **Ziel:** Move in App-Crate.
- **Dateien:** `src-tauri/src/runtime/appearance_init.rs` (neu), `core/src/settings.rs` (entfernt).
- **Risiko:** Niedrig.
- **Akzeptanz:** Build macOS/Windows.

### PR (Frontend): IPC-Registry (F4/F8)
- **Ziel:** `shared/ipc/{commands,events,types}.ts`; Event-Strings zentralisiert.
- **Dateien:** neue Registry; Feature-`api.ts` delegieren.
- **Risiko:** Mittel.
- **Akzeptanz:** `verify` grün; CI-Check Event-Strings.

### PR (Infra): Automatisierbare Regeln (cargo deny, depcruise, Event-String-Check)
- **Ziel:** Regeln aktivieren.
- **Dateien:** `Cargo.toml` (deny config), `.dependency-cruiser.cjs`, `scripts/check-events.sh`.
- **Risiko:** Niedrig.

---

## 11. Risikoanalyse

| Risiko | Wahrsch. | Impact | Mitigation |
|---|---|---|---|
| Regressions bei `activity.rs`-Split | Mittel | Hoch | Schrittweise Untermodule; Re-Exports halten API stabil; Tests + Smoke |
| DB-Schema-Drift | Niedrig | Hoch | Migrations unangetastet; `SCHEMA_VERSION`-Test |
| Frontend-Import-Brüche | Mittel | Mittel | Barrels zuerst; tsc in verify |
| Windows-Build-Bruch | Niedrig | Mittel | Clippy-Target in jedem Rust-PR |
| CLI/RPC-Protokoll-Bruch | Niedrig | Hoch | Protocol-Tests; COMMANDS.md-Review |
| Over-Engineering | Mittel | Mittel | Ports nur an 3 Grenzen; keine neuen Crates |
| Rebrand (Aevio→Aevio) Scope-Creep | Mittel | Mittel | Separate PR; Code-Identifier bleiben bis Rename-PR |

**Rückfall:** Jeder PR ist revertierbar. Keine Verhaltensänderungen → kein Datenmigrations-Rollback. Core-Splits: Re-Exports bleiben, Konsumenten brechen nicht.

---

## 12. Messbare Definition-of-Done-Kriterien

- [ ] `bun run verify` grün auf jedem PR.
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` grün auf jedem Rust-PR.
- [ ] `node --test scripts/*.test.mjs` grün, falls betroffen.
- [ ] Keine Datei > 1500 LOC nach Migration (Ziel ≤ 1000, Ausnahmen begründet).
- [ ] `aevio-core` hat keine `tauri`-Dependency (`cargo deny`).
- [ ] Frontend: keine Cross-Feature-Deep-Imports (`depcruise`).
- [ ] Event-Strings Rust↔TS identisch (Check-Script).
- [ ] DB öffnet sich mit bestehenden Daten (Schema-Version-Test).
- [ ] Windows-Clippy grün.
- [ ] Keine Verhaltensänderung (manueller Smoke: Tracking, Review, Invoices, CLI).
- [ ] `COMMANDS.md` aktuell.
- [ ] Sidecar/RPC-Protokoll-Versionen synchron.

---

## Anhang A: Befund-Index (Datei → Befund → Klasse)

| Datei | Befund | Klasse |
|---|---|---|
| `src-tauri/core/src/activity.rs` (3932) | F1 God-Module | BP |
| `src-tauri/core/src/lib.rs` | F2 Domain-Logik im Crate-Root | BP |
| `src-tauri/src/commands.rs` (1216) | F3 Kopplungspunkt | TS |
| `src/lib/api.ts` (659), `src/lib/types.ts` (734) | F4 Flaschenhals | BP |
| `src/components/EntryReviewPanel.tsx` (1181), `src/pages/Settings.tsx` (1653) | F5 Logik in UI | TS |
| `src-tauri/core/src/state.rs` | F6 Mutex-Bündel (Reader by convention) | BE (Teil-TS) |
| `src-tauri/src/capture/windows.rs:77-78` | F7 RefCell-Statics | TS |
| `src-tauri/src/lib.rs` EVENT_* vs `src/lib/api.ts` | F8 duplizierte Event-Strings | BP |
| `src-tauri/core/src/models.rs` (440) | F9 Typ-Dump | TS |
| `src-tauri/src/lib.rs` appearance_init_script | F10 Präsentation in Settings | TS |
| `src-tauri/src/invoices/*` | F11 PDF/Domain gemischt | TS |
| `src-tauri/src/breaks/*` | F12 Engine/Surface gemischt | TS |
| `src-tauri/core/src/ai/*` + `src-tauri/src/ai/*` | F13 verteilte AI-Grenze | BE |
| `src-tauri/core/src/projects.rs:47,336,382,614` | F14 Regex+DB gemischt | TS |
| `src/hooks/useSettings.tsx` (392) | F15 Context+Provider | OV |
| keine `index.ts` in `src/*` | F16 fehlende Public APIs | BP |

---

## Anhang B: Dependency-Graph (Kurzform)

```
Frontend:  features/* ──▶ shared/ipc/* ──▶ Tauri IPC ──▶ app::commands/* ──▶ core::application/* ──▶ core::domain/*
Core:      application/* ──▶ domain/* + infrastructure/* (Traits)
           infrastructure/* ──▶ domain/* (Impls)
           domain/* ──▶ (nichts aus infra/app/tauri)
App:       commands/* ──▶ core::application/* + app::runtime/* + app::ai::port
           ai::worker ──▶ ai::port (Trait) + core::domain::ai
           capture ──▶ platform impls (cfg_attr) + core::domain (WindowSample)
```

---

*Ende. Kein Code wurde geändert. Keine Branches/Commits/PRs erstellt.*
