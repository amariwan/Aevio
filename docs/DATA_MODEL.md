---
id: ref.data-model-legacy
title: Data Model (legacy)
category: reference
domain: platform
status: implemented
source_paths: []
related_docs: []
platforms: [macos, windows]
verification_commit: working-tree
---


# Aevio – Datenmodell (Übersicht)

> Modul-/Entity-Übersicht auf Basis von `apps/api/src` und `apps/desktop/tauri/src`. Das vollständige SQL-Schema liegt in `crates/aevio-core/src/bootstrap/migrations.rs` und den Coach-SQL-Dateien (`schema.sql`, `brain-schema.sql`, `focus-schema.sql`, `adaptive-schema.sql`). Stand: Working-Tree (v0.9.29).

## Core-Entitäten

### Activity & Capture
- **Segment** – erfasster Arbeitsblock (App, Fenstertitel, URL, Start/Ende, Idle-Flag).
- **Time Entry** – reviewbare Einheit (Status: pending/approved/rejected; Category, Project, Description, Confidence).
- **App** – erkannte Applikation (Bundle-ID, Name, Category/Project-Mapping, Exclusion-Toggle).
- **Category** – Kategorie (Name, Farbe, billable).
- **Idle/Sleep-Events** – schließen Segments.

### Projects & Clients
- **Project** – Name, Client, Status (active/completed/archived), Budget, Due-Date, AI-Hints, Rules.
- **Client** – Name, Vertrag, Revenue/Cost/Margin, Teams.
- **Project Discovery** – Vorschläge aus Repo-Pfaden, Forge-URLs, Editor-Titeln.

### Tasks & Labels
- **Task** – Titel, Notes, Priority, Status, Project-Link.
- **Label** – Tagging für Tasks/Entries.
- **Task Relationships** – Verknüpfungen (blockiert/freigebend), Duplikat-Ähnlichkeit.

### Time Tracking & Timers
- **Timer** – named Stopwatch (Segments, concurrent, restart-persistent).
- **Timesheet** – Projects × Days Grid.
- **Entry Filters / Views** – gespeicherte Ansichten (Saved Views).

### Invoices
- **Invoice** – Draft/Sent/Paid/Voided, Client, Project, Line-Items aus approved billable time, PDF, Quote.
- **Invoice Profile** – Firmeninfos, Logo.
- **Billable Entries** – approved billable project time.

### Finance
- **Rate Card** – Stundensätze pro Projekt/Client/Role.
- **Contract** – Verträge (Retainer, etc.).
- **Capacity Plan** – Kapazitätsplanung (Member Allocations, Urlaub).
- **Finance Dashboard Share** – geteilte Dashboards (revokable).

### Coach (eigenes Schema)
- **Goal** – date-spezifisch, Capacity, Notes, Energy, Planning/Skip-State.
- **Coach Task** – Referenz auf personal/project Task oder neu, Estimate, Priority, Order, Completion, Postponement, Carryover.
- **Task Breakdown** – editierbar/reviewbar.
- **Daily Review** – 5-state Review, Progress (Task-Completion, Goal-Assessment, Work-Duration, Unassigned-Time).
- **Focus Session** – über ActivityStore, Recovery-Context.
- **Memory Note** – explizit/confirmed, editable.
- **Routine Template** – anwendbar max 1x/Tag auf explizite Anfrage.
- **Weekly Aggregate** – lokale Aggregate + previous Week.
- **Coach Settings** – Enablement, Style, on-device AI, Reminders, Quiet Hours, Cooldowns, Timer/Widget-Options.

### Decision-Engine
- **DecisionContext** – bounded summary.
- **Question** – Choice / Noul / Score.
- **Policy** – Routing, Cloud-Opt-in, Keys, Timeouts, Retries.

### Energy
- **Energy Summary** – Energie-Level (Batterie-basiert).
- **Energy History** – historische Energie-Daten.

### Breaks
- **Break State** – aktueller Pausenstatus.
- **Break** – Pausen-Records (list_breaks).

### Automation
- **Routine** – name, prompt, @-mentions, scope (personal/team/workspace), schedule (on-demand/daily/weekdays/weekly/monthly).
- **Run** – Ausführung eines Routine/Report (Brief).

### Reports & Dashboards
- **Report** – Definition + Run (Template/AI, Schedule, Delivery, Export).
- **Dashboard** – gespeicherte Tracked-Time-Abfragen (Favorites, Sharing, Chat).

### Governance
- **Team**, **Department**, **Member**, **Defaults** – Organisationsstruktur, Rollen, Sichtbarkeit.

### Documents, Notes, Comments, Attachments, Mentions
- **Document**, **Note**, **Comment**, **Attachment**, **Mention** – Work-Management-Artefakte.

### Cycles & Initiatives
- **Cycle**, **Initiative** – Planungs-/Roadmap-Entitäten.

### Integrations
- **Integration** – externe Integrationen (Model + CRUD).

### Workspace & Settings
- **Workspace** – Workspace-Modell, Switcher.
- **Settings** – Theme/Accent, Menu-Bar, Close-Behavior, Storage, Retention, Expected Hours, Categories & AI, Breaks, Decision-Settings, Invoice-Profile/Logo.

### Usage & Summary
- **Usage Snapshot** – Nutzungsdaten.
- **Productivity Summary** – offene Summary-Views.

### Pulse & Reminders
- **Pulse Panel** – Reminder-UI.
- **Status** – App-Status.

### IPC / RPC
- **Protocol** – JSON-basiertes IPC-Protokoll (Unix sockets / Windows named pipes).

---

## Datenfluss-Invarianten
- Rust besitzt allen durable State; React hält nur Route, transienten UI-State, letzten Snapshot.
- Geld = Integer-USD-Cents.
- Invoice-PDFs in Rust (printpdf).
- Coach-Brain-Schema (`brain-schema.sql`) für Knowledge-Graph-Grundlage.

## Migrations
- Versionierte SQLite-Migrationen in `crates/aevio-core/src/bootstrap/migrations.rs` (+ tests).
- Coach-Schemas separat: `schema.sql`, `brain-schema.sql`, `focus-schema.sql`, `adaptive-schema.sql`.

## Referenzen
- `crates/aevio-core/src/bootstrap/migrations.rs`
- `crates/aevio-core/src/coach/*.sql`
- `crates/aevio-core/src/lib.rs` (Modul-Liste)
- `docs/features/ai-coach/data-model.md`
