---
id: ref.route-map
title: Route Map
category: reference
domain: platform
status: implemented
source_paths: []
related_docs: []
platforms: [macos, windows]
verification_commit: working-tree
---


# Aevio – Route Map (Frontend)

> Route-Union aus `apps/web/src/lib/types.ts` und Dispatch in `apps/web/src/shell/App.tsx`. Kein Router-Lib; Browser-style History (Stack + Cursor).

## Route-Varianten

| Route | Felder | View |
|---|---|---|
| `dashboard` | – | `Dashboard` |
| `cycles` | – | `Cycles` |
| `initiatives` | – | `Initiatives` |
| `notes` | – | `Notes` |
| `documents` | – | `Documents` |
| `issues` | – | `Issues` |
| `focus` | – | `Focus` |
| `agent` | – | `Agent` |
| `finance` | – | `Finance` |
| `coach` | – | `ProductivityCoach` |
| `catalog` | – | `Catalog` |
| `tasks` | – | `Tasks` |
| `teams` | `tab?` (teams/members/departments/defaults/audit) | `Teams` |
| `calendar` | `scale?`, `date?`, `entryId?`, `review?` | `Calendar` |
| `timesheet` | `scale?`, `date?`, `tab?`, `groupBy?` | `MyTimesheet` |
| `timers` | – | `Timers` |
| `apps` | `scale?`, `date?`, `tab?` (timeline/log), `appId?` | `Apps` |
| `entries` | `range?`, `date?`, `view?`, `filters?`, `groupBy?`, `stackBy?` | `TimeEntries` |
| `timesheets` | `scale?` (day/week), `date?`, `rows?` (project/category), `filters?` | `Timesheets` |
| `projects` | `tab?`, `projectId?`, `range?` | `Projects` |
| `clients` | `tab?`, `clientId?`, `range?` | `Clients` |
| `invoices` | `invoiceId?`, `compose?` ({clientId?, projectId?}) | `Invoices` |
| `settings` | `section?` | `Settings` |

## Sidebar-Gruppen (Navigation)

### Track
- Dashboard (`dashboard`)
- Focus (`focus`)
- Agent (`agent`)
- Coach (`coach`)
- Calendar (`calendar`) – Badge: pendingCount
- My Timesheet (`timesheet`)
- Apps (`apps`)

### Analyze
- Time Entries (`entries`)
- Timesheets (`timesheets`)
- Finance (`finance`)

### Work
- Catalog (`catalog`)
- Tasks (`tasks`)
- Cycles (`cycles`)
- Initiatives (`initiatives`)
- Notes (`notes`)
- Documents (`documents`)
- Issues (`issues`)
- Projects (`projects`)
- Clients (`clients`)
- Invoices (`invoices`)

### Weitere (Teams, Settings über Footer/TopBar)

## Datenfluss
- `nav: { entries: Route[], cursor: number }` – Browser-style History.
- `navigate(next)` pusht, `replace(next)` ersetzt aktuellen Eintrag.
- `currentRoute = nav.entries[nav.cursor]`.
- Deep-Linking über Route-Objekte; keine URL-Router-Lib.

## Verwandte Dateien
- `apps/web/src/lib/types.ts` (Route, EntryFilters, ...)
- `apps/web/src/shell/App.tsx` (Dispatch, NavState)
- `apps/web/src/components/layout/Sidebar.tsx` (Gruppen/NavTab)
- `apps/web/src/components/layout/TopBar.tsx`
