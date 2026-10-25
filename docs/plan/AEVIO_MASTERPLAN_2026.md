# Aevio – Vollständiger Architektur- und Migrationsmasterplan

**Stand:** 21. Oktober 2026  
**Status:** Zielarchitektur / Plan, noch nicht gegen den aktuellen Repository-Stand verifiziert  
**Leitprinzip:** Ein Feature, eine fachliche Implementierung, mehrere Adapter; Server ist immer optional.

## 1. Produkt- und Betriebsziel

Aevio ist eine **local-first Produktivitätsplattform** mit optionalem Self-Hosting. Die Desktop-Anwendung muss nach Installation ohne Account, Server und Internet die vollständigen persönlichen Funktionen ausführen können. Ein Benutzer kann später einen eigenen Server verbinden und bewusst freigegebene Daten zwischen Geräten, Benutzern, Teams und Workspaces synchronisieren.

| Betriebsmodus | Daten | Identität | Funktionen |
|---|---|---|---|
| Standalone | lokale SQLite | lokales Profil, kein Remote-Login | persönliche Funktionen vollständig offline |
| Connected | lokale SQLite + optional selbst gehosteter Server | lokal + Server-Gerätebindung | persönliche Funktionen offline, Teamaktionen serverbestätigt |
| Self-hosted Server | PostgreSQL bevorzugt, SQLite für genau eine Instanz | Server-Accounts und optional OIDC | API, Multi-User, Teams, Rechte, Sync, Jobs |
| Web-Client | via Server-API | Server-Session | gemeinsame Serverfunktionen, keine native Desktop-Capture |

**Nicht verhandelbar:** Keine Hintergrund-Uploads persönlicher Rohaktivität, Fensterdaten, Coach-Lerndaten oder Dateien ohne explizite Freigabe. Alle lokalen Anwendungsfälle bleiben ohne API erreichbar. Server-Unverfügbarkeit darf keinen lokalen Start blockieren. Serverpflichtig sind nur kollaborative Operationen wie Teamfreigaben, Rollenänderungen und verbindliche Rechnungsfinalisierung.

## 2. Architekturentscheidungen und Stack

- Sprache und Runtime: Rust 2024, Tokio auf dem Server; Synchronität lokal, wo sinnvoll.
- HTTP: Axum + Tower, OpenAPI als dokumentierter Vertrag.
- Desktop: Tauri 2, React + TypeScript, native OS-Adapter und bei Bedarf SwiftUI auf macOS.
- Web: vorhandenes React/Next.js-Setup beibehalten, sofern der Ist-Audit keine Ablösung begründet.
- Persistence: SeaORM 2.x async für PostgreSQL; `sea-orm-sync` für lokale SQLite-Zugriffe. SeaORM-Migrationen versioniert.
- Daten: PostgreSQL als Standardserver, SQLite für Desktop/CLI und bewusst begrenzten Single-Instance-Server.
- Sync: bestehenden transaktionalen Outbox-/Cursor-Mechanismus vorerst beibehalten; PowerSync nur nach Sicherheits-, Offline-, Lizenz- und Migrations-POC gegebenenfalls ersetzen.
- Identity: lokale Konten, Argon2id, Sessions; optional OIDC mit Authentik/Keycloak.
- Cache: begrenzter Moka-TTL-Cache; Redis erst bei nachgewiesenem Mehrinstanzbedarf.
- Jobs: bestehende bewährte Queue oder OSS-Bibliothek evaluieren; persistente Jobs, Lease, Retry, Dead Letter.
- Events: Transactional Outbox für zuverlässige Ereignisse; WebSockets nur als Signalweg, niemals als alleinige Nachholquelle.
- Storage: autorisiertes lokales Filesystem und optional S3-kompatibler Adapter.
- Betrieb: Docker Compose, TLS-Reverse-Proxy, Health/Readiness, Backups, OpenTelemetry/Metriken.
- Qualität: Cargo fmt/clippy/nextest, Testcontainers, Frontend-Checks, Contract-, Security-, Recovery- und Migrationstests.

## 3. Zielstruktur – Monorepo

```text
Aevio/
├── apps/
│   ├── api/                         # optionaler Axum-Server
│   ├── desktop/                     # vollständig eigenständige Tauri-App
│   ├── cli/                         # lokale und optionale Remote-CLI
│   └── web/                         # Server-Web-Client
├── crates/
│   ├── aevio-core/                 # minimale fachliche Kernprimitive
│   ├── aevio-application/          # gemeinsame Application-Primitive, keine Feature-Duplikate
│   ├── aevio-services/             # Eigentümer ALLER fachlichen Use Cases
│   ├── aevio-persistence/          # SeaORM + Sync/Async DB-Adapter
│   ├── aevio-infrastructure/       # AI, Secrets, Storage, Mail, Cache, Jobs, Telemetrie
│   └── aevio-contracts/            # versionierte Transport-/Event-/Sync-Verträge
├── packages/
│   ├── ui/                         # Designsystem und zugängliche Komponenten
│   ├── features/                   # gemeinsame UI-Feature-Komposition
│   ├── api-client/                 # generierter/typisierter API-Client
│   ├── types/                      # generierte TS-Typen; kein paralleles Handmodell
│   └── config/                     # TS-Konfigurationen
├── infra/
│   ├── docker/
│   ├── compose/
│   └── observability/
├── docs/
│   ├── architecture/
│   ├── adr/
│   ├── domains/
│   ├── api/
│   ├── database/
│   ├── sync/
│   ├── security/
│   ├── operations/
│   └── agents/
├── scripts/
│   ├── aevio-knowledge.mjs
│   └── project-brain.mjs
├── tests/
│   ├── contract/
│   ├── integration/
│   ├── security/
│   ├── recovery/
│   └── performance/
├── AGENTS.md
├── Cargo.toml
├── package.json
└── justfile
```

Das ist eine **logische Zielkarte**, kein Befehl zur sofortigen Erstellung leerer Ordner. Bereits passende Strukturen bleiben erhalten. Crates werden nur getrennt, wenn dadurch Ownership, Build-Grenzen oder echte Wiederverwendung besser werden.

## 4. Jede App – detaillierte Verantwortung und Struktur

### 4.1 `apps/api` – Axum-Kompositionswurzel

```text
apps/api/
├── Cargo.toml
├── src/
│   ├── main.rs                    # Prozessstart und Exit-Code
│   ├── lib.rs                     # testbare Server-Komposition
│   ├── bootstrap/
│   │   ├── mod.rs
│   │   ├── config.rs              # validierte Runtime-Konfiguration
│   │   ├── dependencies.rs        # Ports mit Implementierungen verbinden
│   │   ├── lifecycle.rs           # Pools, Jobs, Shutdown
│   │   └── readiness.rs
│   ├── http/
│   │   ├── mod.rs
│   │   ├── router.rs
│   │   ├── middleware/            # Auth-Extraction, Request-IDs, Limits, CORS
│   │   ├── handlers/              # thin handlers: Auth, Workspaces, Projects, Tasks, Time, Sync...
│   │   ├── extractors/
│   │   ├── dto/                    # nur Transport-DTOS; generierbar wo möglich
│   │   └── errors.rs
│   ├── realtime/
│   │   ├── websocket.rs
│   │   └── subscriptions.rs
│   └── health/
│       ├── live.rs
│       └── ready.rs
└── tests/
    ├── api_contract.rs
    ├── tenant_isolation.rs
    └── restart_recovery.rs
```

- Besitzt HTTP, Auth-Transport, Konfiguration, Bereitstellung und Server-Lifecycle; **keine Geschäftsregeln**.
- Handler validieren DTOs, konstruieren den autorisierten Request Context, rufen exakt dieselben Use Cases wie Desktop/CLI auf und übersetzen Ergebnisse.
- `/api/v1` kompatibel halten; `/api/v2` nur für nachweisbare Breaking Changes.
- Server-SQLite niemals als unbeschränktes Cluster-Datenbanksystem behandeln; SQLite-Server exakt eine schreibende Instanz.
- Maschinenzugänge haben explizite Workspace-/Feature-Scopes; kein globaler allmächtiger Token.

### 4.2 `apps/desktop` – vollwertiger Standalone-Client

```text
apps/desktop/
├── package.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.tsx
│   ├── app/                     # Desktop-Komposition/Navigation
│   ├── windows/
│   │   ├── main/
│   │   ├── floating-widget/
│   │   ├── quick-actions/
│   │   └── settings/
│   ├── bridge/
│   │   ├── commands.ts          # zentraler, typisierter IPC-Zugriff
│   │   └── events.ts            # zentraler Event-Adapter
│   └── providers/
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/
    └── src/
        ├── main.rs
        ├── lib.rs
        ├── bootstrap/
        │   ├── mod.rs
        │   ├── local_runtime.rs   # SQLite und Use Cases ohne Server starten
        │   └── connectivity.rs    # optionaler Remote-Sync
        ├── commands/             # dünne Tauri-IPC-Adapter
        ├── platform/
        │   ├── macos/
        │   ├── windows/
        │   └── linux/
        ├── capture/              # OS-Beobachtung mit Privacy-Grenzen
        ├── windows/
        ├── tray/
        ├── shortcuts/
        ├── notifications/
        └── secrets/              # native Schlüsselverwaltung
```

- **Boot ohne Server-URL, Auth-Server, Redis, Docker und Internet ist Pflicht.**
- Eigenständige lokale SQLite und lokale Repository-Implementierungen, kein lokales HTTP-Backend als Pflicht.
- Capture, Tray, globale Shortcuts, Window-Management und Notifications bleiben native OS-Adapter.
- Serveranbindung verbindet nur ausgewählte Workspaces; Credentials im OS-Secret-Store, lokale Profil- und Datenhoheit bewahren.
- Trennen des Servers beendet Sync, nicht die lokale App; unerledigte Operationen und Konflikte bleiben sichtbar.
- Frontend importiert `packages/ui`, `packages/features`, `packages/types` und den zentralen IPC-Bridge-Client; keine eigene Kopie der Feature-Logik.

### 4.3 `apps/cli` – lokaler und optionaler Remote-Zugang

```text
apps/cli/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── bootstrap.rs
│   ├── commands/
│   │   ├── auth.rs
│   │   ├── config.rs
│   │   ├── doctor.rs
│   │   ├── projects.rs
│   │   ├── tasks.rs
│   │   ├── time_tracking.rs
│   │   └── sync.rs
│   └── output/
│       ├── json.rs
│       └── table.rs
└── tests/
```

- Nutzt lokale Use Cases in `local`-Mode und vorhandene API-Clients in `remote`-Mode.
- Konsistente Exit-Codes, JSON-Ausgabe, redigierte Fehler, nichtinteraktive Scripts.
- Keine zweite Zeiterfassungs-, Projekt- oder Berechtigungs-Implementierung.

### 4.4 `apps/web` – Team-/Server-Weboberfläche

```text
apps/web/
├── package.json
├── next.config.ts
├── src/
│   ├── app/
│   │   ├── (auth)/
│   │   │   ├── login/
│   │   │   └── callback/
│   │   └── (dashboard)/dashboard/
│   │       ├── time-tracking/
│   │       ├── projects/
│   │       ├── tasks/
│   │       ├── documents/
│   │       ├── finance/
│   │       ├── reports/
│   │       ├── team/
│   │       └── settings/
│   ├── app-shell/
│   ├── providers/
│   └── transport/              # API-Client und Session-Anbindung
└── tests/
```

- Serverseitige UI zu kollaborativen Funktionen; benötigt eine erreichbare API für Teamdaten.
- Next.js Route Handlers, sofern eingesetzt, sind nur BFF/Proxy und nie zweite Businesslogik.
- Keine OS-Capture-Funktion durch Browser vortäuschen.
- UI-Feature-Kompositionen nach `packages/features` auslagern, wenn sie Web und Desktop tatsächlich teilen.

## 5. Crates – Besitz und Abhängigkeiten

### `aevio-core`

Kleine stabile Primitive: IDs, Zeitwerte, Value Objects, allgemeine fachliche Fehler. Kein HTTP, SeaORM, Tauri, Dateisystem oder globaler App-State. Nicht zum Sammelbecken für beliebige Funktionen werden lassen.

### `aevio-application`

Nur wiederverwendbare Application-Bausteine: authentisierter Actor/Scope, Transaction-/Clock-Ports, Cancellation, allgemeine Request-/Result-Muster. **Fach-Use-Cases bleiben im jeweiligen Service**, nicht zugleich hier.

### `aevio-services`

```text
aevio-services/src/
├── lib.rs
├── identity/
├── workspaces/
├── projects/
├── tasks/
├── time_tracking/
│   ├── mod.rs
│   ├── domain/
│   │   ├── entry.rs
│   │   ├── rules.rs
│   │   ├── transitions.rs
│   │   └── errors.rs
│   ├── application/
│   │   ├── commands.rs
│   │   ├── queries.rs
│   │   └── ports.rs
│   └── contracts.rs
├── timeline/
├── documents/
├── invoicing/
├── finance/
├── reports/
├── staffing/
├── automation/
├── coach/
├── integrations/
├── preferences/
└── sync/
```

Die obige Unterstruktur illustriert ein komplexes Modul; kleinere Services bleiben flach. Domain-Regeln sind plattformneutral. Ein Service kann andere Services ausschließlich über stabile öffentliche Application-Schnittstellen verwenden. Zyklen auflösen statt gegenseitige Imports zu erzwingen.

### `aevio-persistence`

```text
aevio-persistence/src/
├── lib.rs
├── entities/                  # SeaORM ActiveModels/Entities
├── sqlite/
│   ├── connection.rs
│   ├── migrations.rs
│   ├── repositories/
│   └── transaction.rs
├── postgres/
│   ├── connection.rs
│   ├── migrations.rs
│   ├── repositories/
│   └── transaction.rs
├── mapping/                   # Domain ↔ Persistence; keine Leaks
└── diagnostics/
```

SeaORM für produktive Abfragen; kein dynamisch verkettetes Business-SQL. PRAGMAs, Migrationen, EXPLAIN und zwingende Low-Level-Operationen sind nur als dokumentierte, zentral kontrollierte Ausnahmen zulässig. Ein gemeinsamer Repository-Vertrag bedeutet nicht, dass Sync und Async mit unsicheren Abstraktions-Tricks vereinheitlicht werden müssen: Fachregeln teilen, Ausführungsadapter sauber trennen.

### `aevio-infrastructure`

`ai/`, `cache/`, `storage/`, `email/`, `jobs/`, `events/`, `secrets/`, `observability/`, `configuration/`. Jeder Provider hat einen klaren Adapter und explizite Capabilities. Cloud-AI ist opt-in, vorhandene Ollama-/lokale Entscheidungsmodelle bleiben nutzbar. Keine automatische Cloud-Eskalation.

### `aevio-contracts`

Versionierte externe DTOs, Sync-Operationshüllen, Fehlercodes und Event-Schemas. Domain-Objekte nicht automatisch als öffentliche API serialisieren; TypeScript-Bindings möglichst generieren, DTOs an genau einer Quelle pflegen. API-Feldnamen und Timestamp-Einheiten rückwärtskompatibel halten.

## 6. DRY- und Ownership-Matrix

| Fähigkeit | Einziger fachlicher Eigentümer | Wiederverwendung | Zulässige Unterschiedlichkeit |
|---|---|---|---|
| Zeitberechnung, Split/Merge, Approve | `services/time_tracking` | API, IPC, CLI | Darstellungs-/Transportschicht |
| Projekte und Aufgaben | `services/projects`, `services/tasks` | alle Clients | lokale/Remote-Repository-Adapter |
| Rechte | `services/identity`, `services/workspaces` | alle Server-Use-Cases | lokaler Standalone-Scope vs Server-Scope |
| Sync-Konflikte/Operationen | `services/sync` | Desktop, API | Local Outbox vs Server-Inbox |
| Rechnungsregeln | `services/invoicing` | lokal, Server | verbindliche Team-Finalisierung online |
| Coach/Entscheidungen | `services/coach` | Desktop und explizite Servercalls | Plattform-Capture und lokale AI-Provider |
| HTTP-Vertrag | `aevio-contracts` + OpenAPI | Web, CLI | serialisierter Transport |
| UI-Funktionen | `packages/features` | Desktop und Web | Native Controls / Route-Shells |
| UI-Bausteine | `packages/ui` | Desktop und Web | plattformspezifische Fenster |
| SeaORM-Schemas | `aevio-persistence` | Server, Desktop | unterschiedliche DB-Dialekte/Migrationen |

**Regel:** Eine fachliche Regel wird genau einmal definiert. Doppelte Adapter sind keine DRY-Verletzung, wenn sie unterschiedliche Plattformverträge implementieren. Keine `shared/utils`-Ablage für fachlich unklare Kopien. Vor Neuerstellung mit ProjectAtlas/GitNexus/rg nach bestehenden Funktionen suchen; erst dann extrahieren.

## 7. Abhängigkeits- und Build-Regeln

```text
Apps → Application/Service Public APIs → Domain/Core
Apps → Infrastructure/Persistence (nur Composition Root)
Infrastructure/Persistence → Port-Verträge / Domain-Typen
Contracts → stabile DTOs und primitive Typen
Domain/Core ↛ Apps, Axum, Tauri, SeaORM, Redis, HTTP
Service A ↛ private Module/Repositories von Service B
```

- Bestehende Importpfade bei Migration über schmale `pub use`-Kompatibilitätsschichten erhalten und erst kontrolliert abbauen.
- `mod.rs` als kurze öffentliche Grenze; Dateien um 500 LOC als Review-Signal, keine willkürliche Funktionszersplitterung.
- Architektur-CI mit `cargo metadata`/Dependency-Policy sowie AST-/`rg`-Checks für verbotene Imports, ORM-Leaks und Raw-SQL.
- Für Frontend: `@/` innerhalb einer App; Paketimporte für app-übergreifende Komponenten, keine relativen Imports über Paketgrenzen.
- Container/API/Web dürfen nicht zwingend in den Desktop-Build hineinrutschen.

## 8. Datenhaltung und Vertragsinvarianten

### Lokal

- SQLite im App-Datenverzeichnis, WAL, Foreign Keys, versionierte Migrationen, Backup vor riskanter Migration.
- Lokale persönliche Workspaces funktionieren ohne serverseitige Mitgliedschaft.
- Lokale Rohaktivität, App-/Fenstertitel, Coach-Feedback und private Dokumente sind standardmäßig `local_only`.
- Ein Gerät kann einen persönlichen Workspace später durch einen expliziten Export-/Share-Workflow mit einem Server verknüpfen; keine stillen ID-Kollisionen.

### Server

- Jede geteilte Entität hat `workspace_id`, globale Sync-ID, Revision und Audit-/Tombstone-Regeln, wo fachlich erforderlich.
- Role-based Access plus ressourcenbezogene Checks im Use Case UND in Repository-Scopes; keine unautorisierte Resource-ID als globale Suche.
- Workspace-weite Revisionszähler werden transaktional fortgeschrieben, nicht durch Zeitstempel geraten.
- Fachänderung + Revision + Outbox-Event im selben DB-Commit.
- Idempotency-Key mit Operation-Hash, Actor, Workspace und Ablaufpolitik. Gleiches Key + anderer Payload = Konflikt.
- Geld in Integer-Minor-Units, Zeitzonen als UTC-Zeitstempel plus gesonderte Anzeigezone, globale UUIDs für Sync.

### Synchronisationsprotokoll

1. Lokale Änderung und Outbox-Operation atomar speichern.
2. Bei Verbindung Gerät/Account authentifizieren und Workspace-Scopes abrufen.
3. Server prüft Typ, Berechtigung, Revision, fachliche Invarianten und Idempotenz.
4. Server committed Domain-Änderung, Revision und Outbox gemeinsam.
5. Client bestätigt Upload und zieht Änderungen über Cursor nach – getrennte Fortschrittsmarker.
6. Konflikte gehen in eine sichtbare Resolution-Queue; lokale Änderungen nicht heimlich überschreiben.
7. Tombstones verhindern Wiederauferstehung gelöschter Daten; Cursor-Ablauf führt zu sicherem Snapshot-Rebuild.

Sync-Payloads nur als registrierte typisierte Fachoperationen. Keine generische SQL-Replikation oder unkontrollierte Tabellen-Patches über die API.

## 9. Auth, Datenschutz und Sicherheit

- Standalone startet mit lokalem Profil; Server-Login nur bei Verbindungswunsch.
- Passwort-Hashing Argon2id, gehashte zufällige Session-Tokens, Refresh-Rotation, Revocation und Recovery-Prozesse.
- OIDC Authorization Code + PKCE, Issuer/Audience/State/Nonce; Accounts anhand `(issuer, subject)` statt Email automatisch zusammenführen.
- Rollen `owner`, `admin`, `member`, `viewer`; letzter Owner nicht entfernbar; Service Tokens mit expliziten Rechten.
- Rate Limits, sichere CORS/CSRF-Cookie-Konfiguration je Client, Request-Größen, Zeitlimits, Secret-Redaktion.
- Tauri-Capability-/IPC-Rechte minimal; keine beliebigen Dateipfade aus Frontend an Dateisystem durchreichen.
- Export, Widerruf, Löschung und Trennung vom Server sind getrennte explizite Workflows; lokale Daten werden bei Logout nicht still gelöscht.
- Keine Telemetrie mit personenbezogenen Inhaltsdaten; Logs mit Request-/Job-IDs und sicheren Fehlermeldungen.

## 10. API, UI und Provider

- API: `/api/v1` Bestand, OpenAPI, stabile Fehlercodes und Request-ID; `v2` nur bei Inkompatibilität.
- Desktop: typed Tauri IPC Commands/Events, gebündelte Bridge, stabile Names/Contracts, keine wiederholten Fetch-Implementierungen.
- CLI: JSON/human output aus denselben Use Cases; optional Remote-Modus über generierten Client.
- UI: `packages/ui` enthält primitives Designsystem; `packages/features` enthält wiederverwendbare Formulare, Tabellen, State/Queries ohne plattformspezifische Capture-Logik.
- AI: lokale Ollama-Modelle und bestehende Regeln/Sidecar priorisieren, Provider-Konfiguration explizit; sensible Daten nie ohne Opt-in in Cloud-Endpunkte.
- Storage: namespace-/workspace-bezogene Autorisierung vor Download; gestreamte Dateien, Size-Limits, Integritätsprüfung und sauberer Garbage-Collection-Lifecycle.
- Jobs: durable queues mit Retry, Backoff, Dead-Letter und Admin-Sichtbarkeit; Memory-In-Process nur für verlierbare Aufgaben.
- Cache: nie Source of Truth, explizite Invalidation; bei Security-Quotas im Ausfall fail closed.

## 11. Umsetzung und Abnahme ohne künstliche Stopps

| Reihenfolge | Arbeit | Exit-Kriterium |
|---|---|---|
| 0. Baseline | Repo/AGENTS/Policies lesen, `git status`, Tests, Graphify/GitNexus Aktualität prüfen | Matrix aller aktuellen Apps/Features/Routes/Commands/Schemas |
| 1. Contracts | IDs, DTOs, Auth Context, Ports, API-/IPC-Kompatibilität katalogisieren | Contract-Snapshots grün |
| 2. Composition | gemeinsam benutzte Domain/Application sauber extrahieren, Adapter dünn machen | Desktop startet ohne Server, Server/CLI kompiliert |
| 3. Persistence | SeaORM-Policy, SQLite/PG-Adapter, versionierte Migrationen | echte DB-Contract- und Migrationstests grün |
| 4. Identity/Sync | Server-Rechte, Geräte, Outbox, Konflikte; PowerSync isolierter POC | Isolation/Offline/Retry/Recovery grün; Technologieentscheidung dokumentiert |
| 5. Domains | Projekte/Tasks → Time/Docs → Billing/Finance/Reports → Coach/Rest | Features paritätisch, alte Adapterkompatibilität bewahrt |
| 6. Operations | Jobs, Provider, Security, Telemetrie, Backup/Restore, Compose | Störfälle/Restore getestet |
| 7. Cleanup | tote Implementierungen nur nach Nutzungsprüfung entfernen; Docs aktualisieren | Gate-Suite grün, keine unbekannten Regressionsfehler |

Die Etappen sind **Arbeitsreihenfolge und CI-Gates, keine Nutzer-Freigabepausen**. Große riskante Änderungen in sichere, reversible Schritte zerlegen. Vor destruktiven Aktionen oder fehlenden Berechtigungen explizit stoppen.

## 12. Verifikationsmatrix

| Szenario | Nachweis |
|---|---|
| Desktop ohne Netzwerk oder API | Installation, Start, Timer, Projekte, Aufgaben, Coach, lokale Persistenz |
| Desktop nach Serververbindung | explizite Auswahl, Upload/Download nur freigegebener Daten |
| Abbruch während Sync | Idempotenz, lokale Outbox intakt, Cursor-Fortsetzung, keine Doppelzählung |
| zwei Benutzer / zwei Workspaces | keine Cross-Tenant-Daten oder privilegierten Operationen |
| Offline-Rollenänderung | Teamaktion blockiert, spätere Ablehnung transparent |
| gleichzeitige Änderungen | deterministische Revisionen, sichtbare Konfliktauflösung |
| Rechnungen | Integer-Cents, Nummernvergabe/Finalisierung online und atomar |
| SQLite → Migration | bestehende DB, WAL, user_version, Daten und Constraints erhalten |
| PostgreSQL Integration | echte PostgreSQL-Tests, nicht durch SQLite simuliert |
| Provider-Ausfall | Readiness / Retry / kein stiller Providerwechsel |
| Server-Neustart / Backup | Rebuild, Outbox-Verarbeitung und Filesystem-Referenzen intakt |
| Öffentliche Verträge | bestehende HTTP-/IPC-/CLI-Aufrufer funktionieren |
| DRY/Architektur | keine doppelte Fachimplementierung, keine Domain-Framework-Abhängigkeit, keine ungenehmigte Business-SQL |

Verifikationsbefehle repositoryabhängig: `cargo fmt --check`, `cargo clippy --workspace --all-targets`, `cargo nextest run`, `bun run fix`, `bun run verify` und gezielte DB-/Plattform-Integrationstests. `graphify update .` nach Codeänderungen, sofern installiert. Native WebView- und Zielhardware-Tests nicht als bestanden markieren, wenn sie nicht ausgeführt wurden.

## 13. Dokumentation und Agent Workflow

```text
docs/
├── architecture/OVERVIEW.md
├── architecture/DEPENDENCIES.md
├── architecture/FEATURE_OWNERSHIP.md
├── architecture/TECHNOLOGY_EVALUATION.md
├── adr/
├── domains/README.md
├── database/SCHEMA.md
├── database/MIGRATIONS.md
├── sync/ARCHITECTURE.md
├── security/SECURITY_MODEL.md
├── api/README.md
├── operations/DEPLOYMENT.md
├── operations/BACKUP_RECOVERY.md
└── agents/ROUTING.md
```

Repository-Vertrag `AGENTS.md`; portable Skill `aevio-engineering-system`. Workflow: Intent klassifizieren → enges Retrieval via `node scripts/aevio-knowledge.mjs context "<task>" --budget=4000` → ProjectAtlas/GitNexus Impact → gezielter Edit → kanonische Tests → Graphify-Update → Evidenzbericht. `UNKNOWN` Impact bleibt ungeklärt; nicht als sicher interpretieren. Keine Agent-Parallelisierung für überlappende Schreibpfade.

## 14. Architektur-Gates und strenges DRY

1. Jede Business-Invariante hat genau **einen fachlichen Eigentümer**.
2. Transportcodes (`apps/api`, `apps/desktop`, `apps/cli`) dürfen nur mappen, validieren und dispatchen.
3. Keine Kopien von Auth-/Permission-/Time-/Finance-Regeln in React, Tauri oder Handlern.
4. Gemeinsame UI-Logik und Typen über Pakete/generierte Verträge, aber keine Web-Abhängigkeiten in Rust-Domain.
5. Kein generisches Repository-Framework nur zur Vermeidung kleiner Adapter-Duplikate.
6. SeaORM einziges produktives Business-Persistence-API; dokumentierte Low-Level-Ausnahmen.
7. Domain und Application enthalten keine Framework-/ORM-SDK-Typen.
8. Modul-Dateien ungefähr 500 LOC, `mod.rs` dünn; große Ausnahmen im Review begründen.
9. Breaking Changes nur mit Kompatibilitätsadapter und Migrationsplan.
10. Bestehende Nutzerdaten und uncommittete Änderungen niemals überschreiben.

## 15. Definition of Done und handoff

- Lokal vollständig lauffähig ohne Serverkonto, Serverprozess oder Internet.
- Optionaler selbst gehosteter Server bietet authentifizierte Multi-User-/Workspace-API und verlässlichen Sync.
- Jede fachliche Regel nur einmal implementiert; API/IPC/CLI sind Adapter auf dieselben Use Cases.
- Datenmigrationen verlustfrei, API-/IPC-/CLI-Kompatibilität nachgewiesen.
- Relevante Tests mit tatsächlichen Messwerten grün; bekannte Baseline-Fehler getrennt dokumentiert.
- Technologiewechsel nur nach belegter Nutzen-/Risikoentscheidung (insbesondere PowerSync).
- Doku, Agent-Knowledge und Graph-Indizes aktualisiert.
- Finalbericht: geänderte Pfade, Regeln, Verifikation, Benchmarks, Risiken, GO/NO-GO. Keine automatischen Commits/PRs/Pushes oder Deployments.

**Architekturentscheidung:** Server und Cloud sind optionale Fähigkeiten. **Standalone ist die Baseline**, und **DRY gilt für Geschäftslogik und Features, nicht für notwendige Plattformadapter**.
