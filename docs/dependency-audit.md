---
id: engineering.dependency-audit
title: Dependency and package audit
category: engineering
domain: dependencies
status: experimental
source_paths:
  - package.json
  - bun.lock
  - apps/web/package.json
  - Cargo.toml
  - Cargo.lock
  - apps/desktop/tauri/Cargo.toml
  - apps/desktop/tauri/Cargo.lock
  - apps/desktop/tauri/swift/Package.swift
  - apps
  - crates
  - scripts
related_docs:
  - docs/DATABASE_COMPATIBILITY.md
  - docs/architecture/monorepo.md
  - docs/architecture/react.md
  - RELEASING.md
platforms: [macos, windows, linux]
verification_commit: working-tree
---

# Dependency- und Package-Audit

Auditstand: 2026-10-24, Arbeitsbaum. Es wurden keine Packages installiert,
aktualisiert oder entfernt. Dieser Bericht ist eine Bestandsaufnahme; Empfehlungen
sind keine ausgeführten Änderungen.

## Ergebnis zuerst

1. Sicherheitsrisiko hoch: cargo audit findet 28 Rust-Advisories. Die produktiv
   relevante Gruppe ist wasmtime 38.0.4 mit 21 Einträgen; außerdem sind zip
   4.6.1, tokio 1.54.0, rustls-webpki 0.103.15, lopdf 0.35.0 und ctutils
   0.4.3 betroffen.
2. JavaScript-Sicherheitsrisiko: bun audit meldet cross-spawn, electron,
   extract-zip und got transitiv. Die Kette kommt insbesondere über das
   deklarierte react-devtools; Electron ist kein Aevio-Runtime-Framework.
3. Direkte JS-Überdeklaration: Root package.json enthält mindestens neun
   Dependencies ohne produktiven Import-/Konfigurationsbeleg:
   @tanstack/react-form, @tanstack/react-query, @tanstack/react-table,
   dice-ui, kbar, nuqs, zod und zustand. Auch das Root-Notification-Package
   sollte gegenüber apps/web/package.json bereinigt werden.
4. Keine voreilige Löschung: Einige scheinbar ungenutzte Packages werden nur
   per CLI, Build-Konfiguration, TypeScript-Referenz, Plugin-Registry oder
   indirekt durch Frameworks verwendet.
5. Größte Optimierungshebel: wasmtime/Cranelift, Tauri/PrintPDF/Image,
   Next/Sentry, Tiptap und die installierte Toolchain. pdfjs-dist ist bereits
   als eigener Lazy-Chunk implementiert.

## Prüfgrenzen und Methode

Geprüft wurden alle gefundenen package.json, bun.lock, Cargo.toml, beide
Cargo.lock und apps/desktop/tauri/swift/Package.swift; tatsächliche Imports in
apps/web/src, Rust-Symbole in apps und crates, Konfigurationen, Scripts, Hooks,
CI und relevante Dokumentation.

Ausgeführt wurden bun pm ls --all, cargo metadata --locked --no-deps,
cargo tree --workspace, cargo audit --json, bun audit --json und bun outdated.
Es wurde kein nativer Tauri-Webview-Check und kein Produktions-Build ausgeführt.
Bundlegrößen sind deshalb statische/Installations-Indikatoren, keine gemessenen
Release-Artefakte.

Bun listete 741 installierte Tree-Knoten. Cargo listete 535 eindeutige
Workspace-Tree-Pakete bzw. 882 Lockfile-Einträge. Diese transitive Menge wird
nicht wegen fehlender Direktimporte als ungenutzt klassifiziert. Die vollständige
Version-/Hash-Inventur bleibt in bun.lock, Cargo.lock und
apps/desktop/tauri/Cargo.lock reproduzierbar.

## Package-Manager und Workspaces

| Bereich | Quelle | Befund |
|---|---|---|
| Bun/npm | package.json, bun.lock | Bun 1.4.3; Root plus apps/* und packages/*; 741 installierte Tree-Knoten |
| Rust/Cargo | Cargo.toml, Cargo.lock | Workspace mit API, CLI, Desktop und Crates; 882 Lockfile-Einträge |
| Desktop Rust | apps/desktop/tauri/Cargo.toml und eigener Lockfile | Tauri, native OS-Features, PDF, SQLite, Sidecar-Anbindung |
| SwiftPM | apps/desktop/tauri/swift/Package.swift | Keine externen Swift-Package-Abhängigkeiten; FoundationModels weak-linked |
| Sonstige | Repository-Suche | Kein package-lock.json, pnpm-lock.yaml, yarn.lock, go.mod, pyproject.toml, Podfile oder Package.resolved |

Interne TS-Packages @aevio/api-client, @aevio/config, @aevio/types,
@aevio/ui und @aevio/web-shared deklarieren keine Dependencies und haben im
geprüften Web-Importgraphen keinen produktiven Consumer-Beleg. Das ist kein
Löschvorschlag. @aevio/plugin-sdk wird durch apps/web/src/lib/pluginHost.ts:2-14
konsumiert und besitzt den lokalen Workspace-Stand 1.0.0.

## Direkte JavaScript-/Bun-Packages

Versionen stammen aus bun pm ls --all. workspace:* bezeichnet ein lokales
Workspace-Package. Status beschreibt den Beleg im aktuellen Tree.

| Package | Version | Verwendung | Einsatzort | Status | Empfehlung |
|---|---:|---|---|---|---|
| @sentry/nextjs | 11.9.0 | Client-Fehler, Replay, Tracing | apps/web/instrumentation-client.ts:1-12 | verwendet | Beibehalten; DSN-gated und Client-only |
| @tauri-apps/plugin-notification | 2.7.0 | Notification-Permission/API | apps/web/src/features/coach/components/Settings.tsx:3 | verwendet | Ownership zwischen Root und Web normalisieren |
| recharts | 3.10.1 | React-Diagramme und Tooltips | apps/web/src/components/evilcharts/** | verwendet | Beibehalten; nicht initial sichtbare Charts lazy laden |
| @tanstack/react-form | 1.33.5 | Formularzustand/Validierung | Root package.json:38 | kein Produktivbeleg | Erst nach Plugin-/Typecheck-Suche entfernen |
| @tanstack/react-query | 5.104.5 | Server-State-Cache | Root package.json:39 | kein Produktivbeleg | Entfernen oder bewusst einführen; aktueller IPC-Fluss ist eigenständig |
| @tanstack/react-table | 9.3.1 | Tabellenmodellierung | Root package.json:40 | kein Produktivbeleg | Nach UI-/Story-/Plugin-Suche entfernen |
| dice-ui | 0.1.1 | UI-Komponenten | Root package.json:42 | kein Produktivbeleg | Prüfen; shadcn/Base UI ist die etablierte UI-Schicht |
| kbar | 1.0.0 | Command Palette | Root package.json:43 | kein Produktivbeleg | Nach dynamischer Palette außerhalb apps/web/src suchen |
| nuqs | 2.10.2 | URL-Query-State | Root package.json:44 | kein Produktivbeleg | Entfernen oder gezielt in Router-Flows einsetzen |
| zod | 4.6.7 | Runtime-Schema-Validierung | Root package.json:46 | kein Produktivbeleg | Vor Entfernung generierte Contracts/Plugin-Runtime prüfen |
| zustand | 5.0.16 | Client-State | Root package.json:47 | kein Produktivbeleg | Entfernen, wenn externe Plugin-Module es nicht importieren |
| @next/bundle-analyzer | 16.4.3 | Bundle-Analyse | Root devDependency; DEV_TOOLS.md | nicht konfiguriert sichtbar | Entfernen oder next.config.ts explizit anschließen |
| @tauri-apps/cli | 2.12.4 | tauri dev/build CLI | scripts/*.sh, .github/workflows/release.yml | verwendet | Beibehalten |
| husky | 9.1.7 | Git-Hook-Installation | Root prepare, .husky/pre-commit | verwendet | Beibehalten, sofern Hooks gewünscht |
| lint-staged | 17.6.0 | staged Lint/Format | .husky/pre-commit, Root lint-staged | verwendet | Beibehalten |
| oxfmt | 0.74.0 | Formatprüfung/-fix | scripts/verify.sh:16-22 | verwendet | Beibehalten |
| oxlint | 1.89.0 | Frontend-Linting | scripts/verify.sh:16-22, apps/web/oxlint.json | verwendet | Beibehalten |
| react-devtools | 8.0.0 | manuelles React-Debugging | DEV_TOOLS.md | dev-only verwendet | Optional machen oder entfernen; Electron-Kette beachten |
| turbo | 2.11.10 | Workspace-Orchestrierung/Cache | Root Scripts, turbo.json | verwendet | Beibehalten |
| typescript | 6.0.3 | Typechecking/Compiler | apps/web/next-env.d.ts, scripts/verify.sh | verwendet | Major 7 separat gegen Next/Tauri testen |
| @aevio/plugin-sdk | workspace 1.0.0 | Plugin-Host-Vertrag | apps/web/src/lib/pluginHost.ts:2-14 | verwendet | Beibehalten |
| @base-ui/react | 1.9.0 | primitive UI-Komponenten | apps/web/src/components/ui/*.tsx | verwendet | Beibehalten |
| @paper-design/shaders-react | 0.0.82 | Liquid-Metal-Visual | apps/web/src/components/shadcn-space/button/button-34.tsx | verwendet, begrenzt | Beibehalten; Effekt optional/lazy prüfen |
| @tauri-apps/api | 2.12.4 | IPC, Events, Core | apps/web/src/lib/tauri.ts, shell/*, Widget.tsx | verwendet | Beibehalten |
| @tauri-apps/plugin-opener | 2.7.2 | URLs, Pfade, Dateien öffnen | ai, settings, invoices, entries | verwendet | Beibehalten; URI-/Pfadvalidierung |
| @tiptap/extension-* | 3.32.0 | Notes-Editor-Funktionen | apps/web/src/features/notes/Notes.tsx:3-13 | verwendet | Versionen synchron halten; tote Einzel-Extensions prüfen |
| @tiptap/pm | 3.32.0 | ProseMirror-Basis | Tiptap-Transitiv/Manifest | kein direkter Import | Nur entfernen, wenn Tiptap es nicht benötigt |
| @tiptap/react | 3.32.0 | React-Editor-Bindings | Notes.tsx:3 | verwendet | Beibehalten |
| @tiptap/starter-kit | 3.32.0 | Basis-Editor-Extensions | Notes.tsx:8 | verwendet | Mit expliziten Extensions auf Doppelungen prüfen |
| class-variance-authority | 0.7.1 | Varianten-/Klassenkonfiguration | components/ui, kokonutui | verwendet | Beibehalten |
| clsx | 2.1.1 | Klassenlisten | apps/web/src/lib/utils.ts:1-5 | verwendet | Beibehalten |
| cn | 0.5.0 | externe Klassen-Utility | Manifest; lokales lib/utils.ts definiert eigenes cn | schwacher Beleg | CLI-Requirement prüfen; lokales cn nicht ersetzen |
| lucide-react | 1.59.0 | Icons | UI breit, z.B. NotificationCenter.tsx | verwendet | Einzelimporte und Tree-Shaking beibehalten |
| motion | 14.4.0 | Animationen/Reduced Motion | apps/web/src/components/** und Features | verwendet | CSS für einfache Übergänge bevorzugen |
| next | 16.4.2 resolved | Static Export, Dev, Types | apps/web/package.json, next-env.d.ts | verwendet | Lockstand und deklarierte Range dokumentieren |
| pdfjs-dist | 6.4.299 | PDF-Preview | PdfViewer.tsx:1-24 | verwendet, lazy | Beibehalten; bestehendes Lazy-Worker-Muster nutzen |
| react | 19.3.1 | UI-Runtime | alle React-Komponenten | verwendet | Mit react-dom synchron halten |
| react-dom | 19.3.1 | DOM-Renderer | primitives und App-Runtime | verwendet | Beibehalten |
| shadcn | 4.22.1 | Registry/Generator/Tailwind-Bridge | components.json, app.css | build/dev verwendet | Beibehalten; nicht als Runtime-UI verwechseln |
| tailwind-merge | 3.7.0 | Utility-Konfliktauflösung | apps/web/src/lib/utils.ts:2-5 | verwendet | Beibehalten |
| tw-animate-css | 1.4.0 | Tailwind-Animationsutilities | apps/web/src/styles/app.css | verwendet | Beibehalten |
| @biomejs/biome | 2.5.15 | Lint-Suppression-Konventionen/Tooling-Kandidat | keine CLI-/Config-Ausführung gefunden | schwacher dev-Beleg | Entfernen oder bewusst etablieren; doppelte Toolchain |
| @tailwindcss/postcss | 4.3.3 | PostCSS-Plugin | apps/web/postcss.config.mjs:1-3 | verwendet | Beibehalten |
| @types/node | 22.20.5 | Node-Typen | Next/TypeScript-Toolchain | typing | Beibehalten, wenn Typecheck benötigt |
| @types/react / @types/react-dom | 19.3.0 | React-Typen | TypeScript | typing | Beibehalten und synchron halten |
| tailwindcss | 4.3.3 | CSS-Compiler | postcss.config.mjs, app.css | verwendet | Beibehalten |

Tatsächliche Beispiele: @aevio/plugin-sdk wird in pluginHost.ts importiert,
Tiptap-Extensions in Notes.tsx, pdfjs-dist dynamisch in PdfViewer.tsx,
Sentry in instrumentation-client.ts, und clsx/tailwind-merge in lib/utils.ts.
Die Root-Pakete @tanstack/*, kbar, dice-ui, nuqs, zod und zustand hatten
dagegen im produktiven apps-/packages-Tree keinen Importbeleg.

### JS-Doppelungen

- @base-ui/react, Radix-Transitives und shadcn berühren dieselbe UI-Schicht,
  haben aber unterschiedliche Rollen: Base UI ist Runtime, shadcn Registry/
  Generator. Radix-Transitives nicht blind entfernen.
- motion und CSS/Tailwind überschneiden sich nur bei einfachen Übergängen.
- recharts und lokale chart-Komponenten sind zwei Visualisierungsschichten;
  Recharts ist real verwendet.
- @tiptap/starter-kit kann Extensions überschneiden, die explizit in Notes.tsx
  geladen werden. Dies ist vor einer Entfernung mit Tiptap-Optionsprüfung zu
  verifizieren.

## Direkte Rust-Crates

Versionen stammen aus cargo tree --workspace. Plattform-Crates werden nur für
das jeweilige Target aufgelöst.

| Package | Version | Verwendung | Einsatzort | Status | Empfehlung |
|---|---:|---|---|---|---|
| tauri | 2.12.4 | Desktop-Runtime, Tray, Webview, IPC | apps/desktop/tauri/Cargo.toml:27, src/lib.rs | verwendet | Beibehalten |
| tauri-build | 2.7.3 | Build-Script | Cargo.toml:18-19, build.rs | verwendet | Beibehalten |
| tauri-plugin-opener | 2.7.2 | native URL-/Pfadöffnung | Rust-Pluginregistrierung | verwendet | Beibehalten |
| tauri-plugin-dialog | 2.8.3 | native Dialoge | src/lib.rs, Commands | verwendet | Beibehalten |
| tauri-plugin-notification | 2.7.0 | native OS-Notifications | src/lib.rs, Notification-Commands | verwendet | Beibehalten |
| tauri-plugin-updater | 2.14.0 | App-Updates | src/lib.rs, tauri.conf.json | verwendet | Signatur-/Endpoint-Policy prüfen |
| tauri-plugin-single-instance | 2.5.4 | ein Prozess je Datenverzeichnis | src/lib.rs, Cargo-Kommentar | verwendet | Beibehalten |
| rusqlite | 0.38.0 | SQLite-/Kompatibilitätszugriff | Persistence, Infrastructure, CLI-Tests | verwendet, mehrfach | Erst nach Adapter-/Kompatibilitätstests reduzieren |
| sea-orm-sync | 2.0.4 | synchroner ORM-Adapter | aevio-persistence, aevio-services | verwendet | Beibehalten |
| sea-orm | 2.0.4 | PostgreSQL-Adapter und Tests | postgres-adapter, API dev-dependency | verwendet | Features minimal halten |
| printpdf | 0.8.2 pinned | PDF-Erzeugung | apps/desktop/tauri/src/invoices/pdf.rs | verwendet | Pin begründen; lopdf-Transitivpfad prüfen |
| image | 0.25.10 | Logo decode/resize/re-encode | invoices, Cargo-Kommentar | verwendet | png/jpeg-Features bereits begrenzt |
| wasmtime | 38.0.4 | Plugin-/Wasm-Sandbox, Cranelift | crates/aevio-services | verwendet, kritisch | P0: gepatchte Linie und Sandbox-Tests evaluieren |
| zip | 4.6.1 | Archive/Plugin-/Export-Pfade | aevio-cli, Infrastructure | verwendet, kritisch | P0: gepatchte Linie oder sichere Symlink-/Mode-Extraktion |
| reqwest | 0.13.5 | HTTP/Registry/Server-Kommunikation | Services, Desktop | verwendet | Featurepolitik dokumentieren |
| rustls | 0.23.46 | TLS Crypto Provider | Desktop | verwendet | ring-Provider halten; webpki-Lock prüfen |
| tokio | 1.54.0 | Runtime, Timer, Netzwerk | API, Services | verwendet, advisory | P0: ab 1.54.1 evaluieren |
| axum | 0.8.9 | HTTP-Routing | aevio-http, API | verwendet | Beibehalten |
| serde | 1.0.229 | Serialization/Contracts | alle Rust-Schichten | verwendet | Beibehalten |
| serde_json | 1.0.154 | JSON IPC/API/Storage | alle Rust-Schichten | verwendet | Beibehalten |
| uuid | 1.28.0 | v7 IDs, serde | Domain/IPC/DB | verwendet | Beibehalten |
| chrono / chrono-tz | 0.4.45 / 0.10.4 | Zeit, Datum, Zeitzonen | Core/Persistence/Desktop | verwendet | Featureflags minimal |
| regex | 1.13.1 | Parsing/Validierung | Core/Persistence/Desktop | verwendet | Beibehalten |
| argon2 | 0.5.3 | Passwort-/Token-Hashing | Application/Services | verwendet | Parameter-/Secret-Handling prüfen |
| rand_core | 0.6.4 direkt | Zufallswerte | Auth/Token | verwendet | Nicht ohne Crypto-Kompatibilität major-upgraden |
| sha2 | 0.10.9 direkt | Hash/Fingerprint | Contracts/HTTP/Infrastructure | verwendet | Beibehalten |
| base64 | 0.22.1 direkt | Binary/Text-Encoding | API/Infrastructure | verwendet | 0.21/0.23 transitiv zuerst nicht erzwingen |
| ed25519-dalek | 2.2.0 | Signaturprüfung | Infrastructure | verwendet | Beibehalten; Schlüssel nie loggen |
| semver | 1.0.28 | Release-/Plugin-Versionen | Contracts/Services/API | verwendet | Beibehalten |
| url | 2.5.8 direkt | URL parsing/allowlists | Services/Desktop | verwendet | Beibehalten |
| dirs | 6.0.0 direkt | OS-Datenpfade | Infrastructure/Persistence | verwendet | dirs 7 transitiv nicht automatisch zusammenführen |
| interprocess | 2.4.6 | lokale IPC | Infrastructure | verwendet | Beibehalten |
| futures-lite | 2.6.1 | Sync-/Async-Brücke | Services | verwendet | Beibehalten |
| anyhow | 1.0.104 | Fehlerkontext | Services | verwendet | Contracts typisiert lassen |
| user-idle3 | 0.7.3 | Idle-Erkennung | Desktop capture/breaks | verwendet | OS-Fallbacks prüfen |
| clap / clap_complete | 4.6.7 / 4.6.11 | CLI und Shell-Completions | apps/cli | verwendet | Beibehalten |
| windows / windows-registry | 0.61 / 0.6 | Windows native APIs/PATH | Windows target sections | target-abhängig | Featurelisten nicht verbreitern |
| libc | 0.2.x | Unix-native APIs | Unix target sections | target-abhängig | Beibehalten |
| core-foundation*, objc2*, block2 | 0.10/0.8/0.6/0.3 | macOS native APIs | macOS target sections | target-abhängig | Plattformmodule beibehalten |
| tempfile / wat | 3.27.0 / 1.262.0 | Testtempfiles/Wasm-Fixtures | Rust dev-dependencies | test-only | Beibehalten solange Tests existieren |

### Rust-Doppelungen und Transitivität

Mehrere Versionen von base64, rand_core, sha2, dirs, rusqlite, url, rustls und
serde erscheinen im vollständigen Tree. Das ist teils durch Tauri/SeaORM/HTTP-
Ökosysteme unvermeidbar. Erst direkte Constraints und Featureflags vereinheitlichen,
dann cargo tree -d erneut bewerten.

rusqlite direkt und über sea-orm-sync ist eine Architekturentscheidung, keine
automatisch ungenutzte Doppelung. Entfernen erst nach produktiver Adapter- und
Kompatibilitätsverifikation gemäß docs/DATABASE_COMPATIBILITY.md.

## Sicherheits-, Wartungs- und Kompatibilitätsbefunde

### npm/Bun

bun audit --json am 2026-10-24 meldete:

| Paket | Herkunft | Befund | Risiko |
|---|---|---|---|
| cross-spawn | transitiv über react-devtools/Tooling | GHSA-3xgq-45jj-v275, ReDoS, high; betroffen unter 6.0.6 | Dev-/Tooling-Risiko |
| electron | transitiv über react-devtools 8.0.0 | 47 GitHub-Advisories, mehrere high; nicht Aevio-Runtime | hohe Installationsfläche |
| extract-zip | transitiv | zwei high Advisories zu Symlink-/Arbitrary-Write-Extraktion | Archive nur sicher extrahieren |
| got | transitiv | GHSA-pfrx-2q88-qq97, Redirect zu Unix-Socket, moderate | Netzwerkpfade prüfen |

Die Advisory-IDs/Links stammen aus dem Audit-JSON: GHSA-3xgq-45jj-v275,
GHSA-jmr9-qjv8-65gv, GHSA-7pqw-9j4j-h8q3 und GHSA-pfrx-2q88-qq97. Vor einem
Fix muss der Audit-Lauf live wiederholt werden.

bun outdated meldete nur typescript 6.0.3 -> 7.0.2. Die übrigen geprüften
direkten Bun-Packages hatten laut diesem Check keinen neueren Latest-Eintrag.
Das ist kein Kompatibilitätsnachweis für ein Major-Upgrade.

### Rust/Cargo

cargo audit --json verwendete eine Advisory-Datenbank mit Stand 2026-10-24
10:30 CEST, 1.352 Advisory-Einträgen, und fand 28 Vulnerabilities:

- wasmtime 38.0.4: 21 Advisories, darunter Sandbox-Escape, OOB-/Crash-,
  Resource-Exhaustion- und Cranelift-Miscompilation-Befunde. Gepatchte Linien
  variieren je Advisory; ein blindes Semver-Update ist nicht ausreichend.
- zip 4.6.1: drei Advisories zu SUID/SGID-Erhalt und Symlink-Extraktion;
  Audit nennt eine gepatchte Major-Linie ab 9.0.0.
- tokio 1.54.0: Race Condition in time::Interval, gepatcht ab 1.54.1.
- rustls-webpki 0.103.15: Certificate-Verification-Befund, gepatcht ab 0.103.16.
- lopdf 0.35.0: Stack Overflow durch tief verschachtelte PDF-Objekte; Audit
  nennt 0.42.0 als Fix. Der transitive Parserpfad von printpdf muss konkret
  verifiziert werden.
- ctutils 0.4.3: Constant-Time-/u128-Maskierungsproblem, gepatcht ab 0.4.4.

cargo audit meldete keine unmaintained-/unsound-Einträge. Das ist nur der
Befund dieses Laufs, kein allgemeiner Wartbarkeitsnachweis.

## Performance, Bundle und Build-Zeit

- node_modules belegt lokal rund 2.0 GB inklusive Plattformvarianten und
  Tooling. Das ist kein Release-Bundlemaß.
- Next, Sentry, Tiptap, Recharts, pdfjs-dist, shadcn-CLI und die oxc-/Biome-
  Toolchain dominieren voraussichtlich Installations-/Build-Arbeit.
- pdfjs-dist ist bereits dynamisch importiert und setzt den Worker in
  PdfViewer.tsx:12-24; das ist gutes Initial-Load-Verhalten.
- Recharts ist tree-shakable, aber die vielen Chart-Wrapper werden über die
  normale Web-App importiert. Nicht initial sichtbare Charts sollten nach
  Messung per next/dynamic gesplittet werden.
- Tauri/PrintPDF/Image/Wasmtime sind Native-/Desktop-Kosten und werden nicht
  durch Web-Tree-Shaking reduziert. default-features = false ist bei reqwest,
  sea-orm-sync, printpdf und image bereits sinnvoll.
- Cargo-Duplikate erhöhen Compilezeit und Artefaktgröße, sind bei Tauri, sqlx,
  printpdf und OS-Bindings aber oft upstreambedingt. Nur nach cargo tree -d
  plus Release-Buildmessung zusammenführen.

## Priorisierte Optimierungen

| Priorität | Maßnahme | Nutzen | Aufwand | Risiko |
|---|---|---:|---:|---:|
| P0 | wasmtime, zip, tokio, rustls-webpki, ctutils und PDF-Transitives auf gepatchte kompatible Linien evaluieren; Tests ausführen | sehr hoch | mittel-hoch | mittel-hoch |
| P0 | react-devtools aus Standard-/CI-Installation entfernen oder strikt lokal machen; Audit erneut ausführen | hoch | niedrig | niedrig-mittel |
| P1 | Root-JS-Kandidaten einzeln mit rg, Plugin-Suche und Build prüfen; nur dann Manifest/Lockfile bereinigen | mittel-hoch | niedrig | mittel |
| P1 | Root-vs-Web-Ownership für Notification, Recharts und Tooling normalisieren | mittel | niedrig | niedrig-mittel |
| P1 | @biomejs/biome und @next/bundle-analyzer aktiv konfigurieren oder abbauen | mittel | niedrig | niedrig |
| P1 | Tiptap starter-kit gegen explizite Extensions prüfen | mittel | mittel | mittel |
| P2 | Recharts-/Shader-/Tiptap-Feature-Splits messen | mittel | mittel | niedrig-mittel |
| P2 | Cargo-Duplikate durch Feature-/Constraint-Normalisierung reduzieren | mittel | mittel-hoch | mittel-hoch |
| P2 | apps/web/package.json 0.9.35 gegen Root/Rust 0.9.38 als Versionsdrift prüfen | mittel | niedrig | mittel |

## Native Framework-Funktionen statt externer Packages

- Tauri statt Browser-Workarounds: Notifications, Dialoge, Opener,
  Single-Instance und Updater sind korrekt native Rust-/Tauri-Funktionen.
- Einfache Query-/Transient-State-Logik kann mit React Hooks,
  URLSearchParams und Next-Routing gelöst werden; das rechtfertigt die
  Entfernung von nuqs erst nach Route-Flow-Prüfung.
- Dauerhafte Daten, IPC-Verträge, Geld, PDF-Komposition und Security-Checks
  bleiben Rust-owned. zod darf diese Grenze nicht als Ersatz für Rust-
  Verifikation verschleiern.
- Einfache opacity/transform transitions sollten CSS/Tailwind bleiben;
  interaktive SVG-/Layoutanimationen belegen motion.
- dirs, libc, objc2, windows und user-idle3 haben keine gleichwertige portable
  Browser-Alternative in einer Tauri-App; OS-Fallbacks bleiben erforderlich.

## Verifikationsplan vor Änderungen

1. Baseline mit git diff für package.json, apps/web/package.json, bun.lock,
   Cargo.toml, Cargo.lock und apps/desktop/tauri/Cargo.toml sichern; vorhandene
   Dirty-Änderungen nicht überschreiben.
2. Pro Kandidat nur eine begrenzte Manifest-/Lockfile-Änderung durchführen,
   danach bun run fix, bun run verify, fokussierte Frontend-Tests und cargo test
   im betroffenen Crate ausführen.
3. Für Security-Fixes zusätzlich cargo audit, relevante Archive-/Wasm-/PDF-Tests
   und beide Target-Konfigurationen prüfen.
4. Für Bundle-Claims Release-Artefakte separat messen; die Aussagen dieses
   Dokuments sind statisch.
5. Nach jeder Änderung bun audit, cargo audit, cargo tree -d und tatsächliche
   Imports erneut prüfen. Keine Löschung nur wegen fehlender Direktimporte.

## Quellen und reproduzierbare Befehle

    bun pm ls --all
    cargo metadata --format-version 1 --locked --no-deps
    cargo tree --workspace
    cargo tree --workspace -d
    bun audit --json
    cargo audit --json
    bun outdated

Primärquellen im Arbeitsbaum sind package.json, apps/web/package.json,
bun.lock, alle Cargo.toml, Cargo.lock, apps/desktop/tauri/Cargo.lock und
apps/desktop/tauri/swift/Package.swift. Advisory-IDs stammen aus dem am
Auditdatum ausgeführten Output; die Bewertung ist zeitabhängig und vor einem
Fix live zu wiederholen.
+

---

## Vertiefte Revision gegen den aktuellen Arbeitsbaum

Revisionsdatum: 2026-10-24. Die vorherige Fassung wurde nicht als
verlässliche Baseline behandelt, weil package.json, apps/web/package.json,
apps/desktop/tauri/Cargo.toml und Cargo.lock uncommittete Änderungen enthalten.
Die folgenden Statuswerte bedeuten:

- VERIFIZIERT: im aktuellen Code, Manifest, Lockfile oder reproduzierbaren
  Tool-Output direkt belegt.
- WAHRSCHEINLICH: starke Indizien, aber ein Build-/Runtime- oder Target-Check
  fehlt.
- UNGEKLÄRT: der Codepfad oder die Advisory-Bedingung ist nicht vollständig
  nachgewiesen; keine Entfernung oder Sicherheitsfreigabe ableiten.

## Korrekturen zur vorherigen Fassung

| Befund | Revision | Status |
|---|---|---|
| Bun-Inventur | 741 Tree-Knoten, 919 Bun-Lock-Package-Records; die frühere Zahl bleibt für den aktuellen Tree korrekt | VERIFIZIERT |
| Cargo-Inventur | 535 eindeutige Workspace-Tree-Pakete; Root-Lockfile 882 Records, Desktop-Lockfile 666 Records | VERIFIZIERT |
| kbar-Nutzung | Frühere Treffer in apps/desktop/tauri/src/* waren nur das Wort kbar als Teil von taskbar; kein kbar-Import | VERIFIZIERT: ungenutzte direkte Root-Dependency |
| sea-orm-sync | Kein direkter sea_orm_sync-Import ist nötig: die Persistence-Schicht nutzt den von sea-orm-sync bereitgestellten sea_orm-Namespace über aevio_persistence::database::sea_orm | VERIFIZIERT: notwendig |
| zip-Risiko | Aevio ruft nicht ZipArchive::extract auf, sondern liest Einträge mit by_index und schreibt Dateien selbst nach Pfad-/Größenprüfung | VERIFIZIERT: konkrete Advisory-API nicht aufgerufen; Rest-Risiko UNGEKLÄRT |
| tokio-Interval-Advisory | Der Code verwendet Tokio als Runtime, aber keine tokio::time::Interval-API; die vielen Interval-Treffer sind Aevio-Domainwerte oder Browser setInterval | VERIFIZIERT: spezifischer Advisory-Angriffsweg nicht belegt |
| node_modules-Größe | Rund 2.0 GB ist eine Installationsgröße. target belegt lokal rund 68 GB und enthält Build-/Cache-Artefakte; beides ist kein Release-Bundlemaß | VERIFIZIERT |
| Versionen | Root/Rust stehen auf 0.9.38, apps/web auf 0.9.35; dies ist im aktuellen Tree Versionsdrift, nicht automatisch ein Dependency-Defekt | VERIFIZIERT |

## Vollständige Inventur und Ownership

Die direkte Inventur ist in den Tabellen dieses Dokuments vollständig für
package.json, apps/web/package.json und alle Cargo.toml. Die transitive
Inventur ist vollständig in bun.lock, Cargo.lock und
apps/desktop/tauri/Cargo.lock enthalten. Jeder transitive Package-Record hat
dort Version, Registry/Workspace-Herkunft, Checksumme und Parent-Dependency.
Ein transitive Package ohne direkten Source-Import ist deshalb nicht ungenutzt:
seine Verwendung ist der Lockfile-Graph und die API des direkten Parents.

| Eigentümer | Direkte Runtime-Verantwortung | Direkte Tool-/Test-Verantwortung | Beleg | Status |
|---|---|---|---|---|
| apps/web | React, Next, Tauri API/Plugins, Tiptap, Charts, PDF.js, UI-Primitives | TypeScript, Tailwind/PostCSS, Biome-Kandidat | apps/web/package.json, apps/web/src | VERIFIZIERT |
| Root Workspace | Sentry, Recharts, Notification, deklarierte Kandidaten; CLI-/Orchestrierungs-Tooling | Tauri CLI, Turbo, Husky, lint-staged, oxc, React DevTools | package.json, scripts, turbo.json, .husky | VERIFIZIERT |
| apps/desktop/tauri | Tauri, native plugins, reqwest, Rust PDF/Image, SQLite, OS bindings | tauri-build | apps/desktop/tauri/Cargo.toml, src/lib.rs | VERIFIZIERT |
| crates/aevio-core | portable domain/time/validation types | keine native UI-/HTTP-/ORM-Verantwortung | crates/aevio-core, architecture_guard.rs | VERIFIZIERT |
| crates/aevio-persistence | SQLite-/SeaORM-sync Repositories und entities | rusqlite/SeaORM tests | crates/aevio-persistence/src | VERIFIZIERT |
| crates/aevio-services | Plugin lifecycle, Wasm runtime, jobs, remote services | Wasm fixtures/tempfile/wat | crates/aevio-services/src/plugins_*.rs | VERIFIZIERT |
| apps/api plus postgres-adapter | Axum API, PostgreSQL contracts, plugin registry views | http-body-util, tower, SeaORM tests | apps/api/src, tests, postgres-adapter | VERIFIZIERT |
| apps/cli | clap CLI, offline package/installer tools, RPC | offline tests | apps/cli/src | VERIFIZIERT |
| Swift sidecar | Foundation executable; keine externen SwiftPM-Pakete | Swift compiler/toolchain | apps/desktop/tauri/swift/Package.swift | VERIFIZIERT |

### Direkte JS-Packages: vertiefte Einstufung

Die vorherige Tabelle bleibt die Package-Referenz. Die folgenden Ergänzungen
präzisieren Ownership und Entfernungssicherheit:

| Package-Gruppe | Tatsächlicher Beleg | Einstufung | Entfernung |
|---|---|---|---|
| @tanstack/react-form, @tanstack/react-query, @tanstack/react-table | Keine Imports in produktivem apps-/packages-Source; nur Root-Manifest und externe Skill-/Doc-Treffer | VERIFIZIERT ungenutzt im aktuellen Produktivtree; WAHRSCHEINLICH historischer/kommender Bedarf | Kandidat, aber nicht sicher entfernbar ohne lockfile-Neuauflösung plus tsc/verify |
| dice-ui, nuqs, zod, zustand | Keine Produktivimports; Aevio nutzt eigene Hooks, Rust snapshots und eigene Contracts | VERIFIZIERT kein aktueller Produktivimport | Kandidat; Plugin-/Generator-/future workspace consumer bleibt UNGEKLÄRT |
| kbar | Keine Produktivimports; taskbar-Treffer sind Textfalsepositives | VERIFIZIERT ungenutzt | Kandidat mit niedrigem Runtime-Risiko, aber erst nach Build/Lockfile-Prüfung entfernen |
| cn 0.5.0 | lib/utils.ts importiert clsx und tailwind-merge und definiert das lokale cn selbst; shadcn bringt zusätzlich cn 0.2.6 transitiv | VERIFIZIERT direkter cn-Import fehlt; WAHRSCHEINLICH überflüssig | Starker Kandidat; shadcn CLI smoke-test erforderlich |
| @next/bundle-analyzer | Nur DEV_TOOLS.md, keine withBundleAnalyzer-/next.config-Verwendung | VERIFIZIERT nicht konfiguriert | Kandidat; keine Runtime-Auswirkung |
| @biomejs/biome | Keine Biome-CLI-/Config-Nutzung; biome-ignore Kommentare sind kein Package-Aufruf | VERIFIZIERT schwacher dev-Beleg | Kandidat; vorher klären, ob der Kommentarstandard absichtlich behalten wird |
| react-devtools | Direktes devDependency; Electron 23.3.13, extract-zip 2.0.1, cross-spawn 5.1.0 und got-Ketten kommen von diesem Tool | VERIFIZIERT | Stärkster sichere-Reduktion-Kandidat; lokal optional statt Standardinstallation |
| @aevio/plugin-sdk | apps/web/src/lib/pluginHost.ts importiert createTauriPluginHostTransport; tsconfig bindet packages/plugin-sdk/src/index.ts | VERIFIZIERT notwendig | Nicht entfernen |
| @paper-design/shaders-react | button-34.tsx importiert LiquidMetal; direkte Wirkung auf einen einzelnen UI-Button | VERIFIZIERT | Nur als Feature-Split/opt-in prüfen, nicht als ungenutzt |
| Tiptap | Notes.tsx importiert React, starter-kit und acht Einzel-Extensions; @tiptap/pm kein Source-Import | VERIFIZIERT | @tiptap/pm nur über Tiptap-Peer-/Runtime-Graph entfernen, nie direkt blind |
| @tauri-apps/plugin-notification | Settings.tsx importiert Permission-API; Root package.json liefert die hoisted Dependency | VERIFIZIERT Verwendung; Ownership nicht ideal | Ownership normalisieren, nicht Funktion entfernen |
| @types/node, @types/react, @types/react-dom | Kein Runtime-Import erwartet; Next/tsc verwendet Typ-Packages über Compilerauflösung | VERIFIZIERT dev-only/typing | Nicht als ungenutzt löschen |

Es gibt aktuell **keinen als sicher entfernbar klassifizierten Runtime- oder
Cargo-Direct-Dependency-Kandidaten**. Sicher entfernbar bedeutet hier:
Produktivimport, Build-/Plugin-/Target-Nutzung und transitive Parent-Contracts
sind ausgeschlossen und ein fokussierter Test bestätigt die Entfernung. Diese
Bedingung ist im aktuellen Arbeitsbaum für keine Dependency vollständig erfüllt.

## Security-Fix-Matrix: aktuelle Advisories

Die Matrix basiert auf dem frisch ausgeführten cargo audit --json vom
2026-10-24. Die Fix-Versionen sind advisory-spezifisch und keine Freigabe für
ein blindes Update. Angriffsweg beschreibt den tatsächlichen Aevio-Pfad, nicht
nur die Existenz des Packages.

| Advisory | Package | Fix laut Audit | Tatsächlicher Aevio-Pfad | Plattform | Status / Maßnahme |
|---|---|---|---|---|---|
| RUSTSEC-2026-0385 | ctutils 0.4.3 | >=0.4.4 | transitiv über sqlx-postgres/SeaORM; kein eigener ctutils-Aufruf | Server/PostgreSQL, ggf. API | WAHRSCHEINLICH transitiv betroffen; lockfile-Fix evaluieren |
| RUSTSEC-2026-0187 | lopdf 0.35.0 | >=0.42.0 | transitiv über printpdf; Aevio erzeugt PDFs, parst keine fremden PDF-Uploads | Desktop | WAHRSCHEINLICH nicht aus untrusted Input erreichbar; printpdf-kompatible Fixlinie prüfen |
| RUSTSEC-2026-0382 | rustls-webpki 0.103.15 | >=0.103.16 | TLS über reqwest/rustls für AI, Updater, Plugin-Registry, Plugin-Network | Desktop, Server | VERIFIZIERT Netzwerkpfad; P0-Lockfile-Fix |
| RUSTSEC-2026-0384 | tokio 1.54.0 | >=1.54.1 | Tokio-Runtime in API/Services/Desktop; kein tokio::time::Interval-Aufruf gefunden | Desktop, Server | VERIFIZIERT Runtime, UNGEKLÄRT Advisory-Erreichbarkeit; patchen und Regressionstests |
| RUSTSEC-2026-0086 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | bösartiges/fehlerhaftes signiertes oder lokal installiertes Plugin-Wasm trifft Cranelift/Engine | Desktop, Server, CLI indirekt | WAHRSCHEINLICH erreichbar über Plugin-Wasm; P0 Upgrade-/Disable-Entscheidung |
| RUSTSEC-2026-0096 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Cranelift/aarch64 guest heap; Plugin-Komponente wird validiert und ausgeführt | macOS arm64, Windows arm64 falls unterstützt | UNGEKLÄRT tatsächliche Triggerform; arm64 Wasm regression test required |
| RUSTSEC-2026-0222 | wasmtime 38.0.4 | >=36.0.13,<37; >=47.0.3 | Engine-/Store-Type-Index bei Plugin-Ausführung | alle Wasmtime-Ziele | UNGEKLÄRT; Multi-store test required |
| RUSTSEC-2026-0020 | wasmtime 38.0.4 | >=36.0.6,<37; >=40.0.4,<41; >=41.0.4 | Advisory nennt WASI; Aevio verweigert WASI-Imports | alle Wasmtime-Ziele | WAHRSCHEINLICH reduzierte Erreichbarkeit, nicht entwarnt |
| RUSTSEC-2026-0092 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Component-model UTF-16-Pfad; Aevio nutzt Core Wasm ABI, kein Component API-Beleg | alle Wasmtime-Ziele | UNGEKLÄRT, wahrscheinlich eingeschränkter Pfad |
| RUSTSEC-2026-0021 | wasmtime 38.0.4 | >=36.0.6,<37; >=40.0.4,<41; >=41.0.4 | WASI HTTP fields; Aevio verweigert WASI | alle Wasmtime-Ziele | WAHRSCHEINLICH nicht erreichbar, Advisory bleibt offen |
| RUSTSEC-2026-0114 | wasmtime 38.0.4 | >=36.0.8,<37; >=43.0.2,<44; >=44.0.1 | Tabellenallokation durch Plugin-Modul; ResourceLimiter begrenzt Memory, Table-Grenze separat prüfen | alle Wasmtime-Ziele | UNGEKLÄRT; adversarial table test required |
| RUSTSEC-2026-0093 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Component-model String-Transcoding; kein Component API im Host belegt | alle Wasmtime-Ziele | UNGEKLÄRT |
| RUSTSEC-2026-0380 | wasmtime 38.0.4 | >=36.0.19,<37; >=43.0.0 | Cranelift Bit-Packing; Plugin-Wasm wird mit Cranelift kompiliert | alle Wasmtime-Ziele | WAHRSCHEINLICH relevant; P0 |
| RUSTSEC-2026-0316 | wasmtime 38.0.4 | >=36.0.16,<37; >=48.0.3,<49; >=49.0.1 | Component dynamic record lifting; kein Component API-Beleg | alle Wasmtime-Ziele | UNGEKLÄRT |
| RUSTSEC-2026-0087 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Cranelift x86-64 guest instruction; Plugin-Wasm ist input-controlled | x86-64 Desktop/Server | WAHRSCHEINLICH relevant; P0 |
| RUSTSEC-2026-0363 | wasmtime 38.0.4 | >=36.0.18,<37; >=48.0.6,<49; >=49.0.3 | Component handle tables; kein Component API-Beleg | alle Wasmtime-Ziele | UNGEKLÄRT |
| RUSTSEC-2026-0269 | wasmtime 38.0.4 | >=36.0.14,<37; >=46.0.3,<47; >=47.0.4 | Filesystem sandbox path; Aevio verweigert WASI/Filesystem imports | alle Wasmtime-Ziele | WAHRSCHEINLICH reduzierte Erreichbarkeit; nicht als Fix entfallen lassen |
| RUSTSEC-2026-0088 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Pooling allocator; Aevio verwendet Default Engine ohne belegte Pooling-Konfiguration | alle Wasmtime-Ziele | UNGEKLÄRT |
| RUSTSEC-2026-0089 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Winch table.fill; Cargo aktiviert cranelift, kein Winch-Aufruf belegt | alle Wasmtime-Ziele | WAHRSCHEINLICH nicht im aktiven Compilerpfad |
| RUSTSEC-2026-0379 | wasmtime 38.0.4 | >=36.0.19,<37; >=48.0.7,<49; >=49.0.4 | Cranelift aarch64 br_table bei Plugin-Wasm | arm64 | UNGEKLÄRT; arm64 adversarial compile test |
| RUSTSEC-2026-0006 | wasmtime 38.0.4 | >=36.0.5,<37; >=40.0.3,<41; >=41.0.1 | Cranelift x86-64 instruction; Plugin bytes are untrusted before signature policy | x86-64 | WAHRSCHEINLICH relevant |
| RUSTSEC-2026-0094 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Winch table.grow; Winch nicht als aktivierter Direct-Featurepfad belegt | alle Wasmtime-Ziele | WAHRSCHEINLICH eingeschränkt |
| RUSTSEC-2026-0085 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Component flags lifting; kein Component API-Beleg | alle Wasmtime-Ziele | UNGEKLÄRT |
| RUSTSEC-2026-0095 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Winch sandbox memory; Winch nicht belegt | alle Wasmtime-Ziele | WAHRSCHEINLICH eingeschränkt |
| RUSTSEC-2026-0091 | wasmtime 38.0.4 | >=36.0.7,<37; >=42.0.2,<43; >=43.0.1 | Component string transcoding; kein Component API-Beleg | alle Wasmtime-Ziele | UNGEKLÄRT |
| RUSTSEC-2026-0367 | zip 4.6.1 | >=9.0.0 | Aevio liest by_index und schreibt eigene Dateien; ZipArchive::extract wird nicht aufgerufen | Desktop, CLI, Server | VERIFIZIERT konkreter Extract-Angriffsweg nicht aktiv; Parser-/DoS-Rest UNGEKLÄRT |
| RUSTSEC-2026-0368 | zip 4.6.1 | >=9.0.0 | gleiche manuelle Extraktion; validate_package_path, MAX_FILES und MAX_ASSET_BYTES aktiv | Desktop, CLI, Server | VERIFIZIERT Symlink-extract-Pfad nicht aktiv; Upgrade weiter prüfen |
| RUSTSEC-2026-0369 | zip 4.6.1 | >=9.0.0 | keine ZipArchive::extract-/Unix-mode-Nutzung im Aevio-Pfad | Desktop, CLI, Server | VERIFIZIERT konkreter Pfad nicht aktiv; nicht als allgemeine Entwarnung |

Bun audit meldet im aktuellen Tree weiterhin cross-spawn und Electron-Ketten.
Die genaue Herkunft ist verifiziert: react-devtools 8.0.0 zieht electron
23.3.13, extract-zip 2.0.1 und cross-spawn 5.1.0; shadcn zieht zusätzlich
cross-spawn 7.0.6. Die Electron- und extract-zip-Advisories betreffen damit
den Dev-Tool-Installationspfad, nicht die Tauri-Runtime. react-devtools ist
daher ein WAHRSCHEINLICH sicherer Reduktionshebel, aber erst nach lokaler
Dev-Workflow-Abnahme.

### Bun-Security-Fix-Matrix: jede Advisory aus dem Audit-Lauf

Die folgenden IDs sind nicht als 50 unabhängige Aevio-Laufzeitpfade zu lesen:
Sie betreffen einen gemeinsamen transitiven Ursprung. Der Eintrag nennt aber
jede vom Audit gemeldete Advisory, ihre konkrete Herkunft und die belastbare
Maßnahme. Bun lieferte für Electron/extract-zip in diesem Lauf keine
zuverlässige gepatchte Zielversion; deshalb ist dort die Dependency-Kette bzw.
der Advisory-eigene Fixstand vor einer Änderung zu verifizieren.

| Package / aktuelle Version | Advisory(s) | Vulnerable-/Fix-Hinweis | Tatsächlicher Aevio-Angriffsweg | Plattform / Status |
|---|---|---|---|---|
| cross-spawn 5.1.0 | GHSA-3xgq-45jj-v275 | <6.0.6; Fix ab 6.0.6 | Nur react-devtools-/Installations- und Tooling-Prozess; keine Produktlaufzeit | alle Dev-Hosts; VERIFIZIERT Dev-Risiko, WAHRSCHEINLICH durch Entfernen/Aktualisieren von react-devtools lösbar |
| electron 23.3.13 | GHSA-7m48-wc93-9g85; GHSA-6r2x-8pq8-9489; GHSA-vmqv-hx8q-j7mg; GHSA-5rqw-r77c-jp79; GHSA-xj5x-m3f3-5x3h; GHSA-r5p7-gp4j-qhrx; GHSA-3c8v-cfp5-9885; GHSA-xwr5-m59h-vwqr; GHSA-532v-xpq5-8h95; GHSA-mwmh-mq4g-g6gr; GHSA-9w97-2464-8783; GHSA-8337-3p73-46f4; GHSA-jjp3-mq3x-295m; GHSA-jfqx-fxh3-c62j; GHSA-4p4r-m79c-wq3v; GHSA-9899-m83m-qhpj; GHSA-f37v-82c4-4x64; GHSA-f3pv-wv63-48x8; GHSA-9wfr-w7mm-pc7f; GHSA-v3j7-r9gq-3gjw; GHSA-jm7p-cc5g-qwxx; GHSA-m55f-7gqj-fr98; GHSA-5c9j-mhmv-5xgx; GHSA-h7rp-cf8h-j98x; GHSA-x8rc-wpg4-grpf; GHSA-9pf5-hg6p-4pwp; GHSA-pfmc-3mgc-p6fp; GHSA-v64r-4m7r-3mvq; GHSA-v93f-fgjr-hjrj; GHSA-9f4c-93c8-jc8g; GHSA-p2rr-rvmm-c5fp; GHSA-f2r8-jv7c-xqmp; GHSA-ff2p-hmqr-hxm4; GHSA-4f78-qhmw-8j8m; GHSA-hq2x-r82h-9wj4; GHSA-gr2m-v5gq-v685; GHSA-j84w-jfhq-vhvj; GHSA-vv43-5jgx-7qv8; GHSA-9qh4-3jw8-366w; GHSA-fw47-9c62-h3vw; GHSA-2p8c-gr87-2676; GHSA-x562-297c-g43g; GHSA-98mx-cjp9-gchm; GHSA-3qxv-5xgr-q7w9; GHSA-h3v5-h239-pg29; GHSA-997q-gvvc-jwmx; GHSA-jr5w-w6x2-84p9 | Alle betreffen Electron-Funktionen wie ASAR, Renderer/IPC, Protokolle, Berechtigungen, Windows/macOS-Integration oder Offscreen Rendering. Aevio nutzt Tauri, nicht Electron; Electron wird nur über react-devtools installiert | Dev-Hosts; VERIFIZIERT kein Tauri-Produktionspfad, WAHRSCHEINLICH sicherster Fix ist react-devtools aus dem Root-Dev-Set zu entfernen oder auf eine Kette ohne Electron 23 zu wechseln; genaue Electron-Zielversion advisory-spezifisch prüfen |
| extract-zip 2.0.1 | GHSA-jmr9-qjv8-65gv; GHSA-7pqw-9j4j-h8q3 | Audit meldet vulnerable Version; Bun liefert keine Fixversion | Nur Electron-Installationskette; Aevio verarbeitet Produkt-ZIPs mit zip-rs und eigener Pfad-/Größenprüfung, nicht extract-zip | Dev-Hosts; VERIFIZIERT kein Produktpfad, WAHRSCHEINLICH durch react-devtools-Kettenentfernung beseitigbar |
| got 11.8.6 | GHSA-pfrx-2q88-qq97 | Audit meldet vulnerable Version; Bun liefert keine Fixversion | Nur @electron/get während Installation; produktive HTTP-Pfade nutzen reqwest und eigene Plugin-Network-Checks | Dev-Hosts; VERIFIZIERT kein Produktpfad, WAHRSCHEINLICH durch Entfernen von Electron-Kette beseitigbar |

Damit sind 1 cross-spawn-, 47 Electron-, 2 extract-zip- und 1 got-Advisory
aus dem aktuellen Bun-Lauf einzeln referenziert (insgesamt 51 Advisories). Die Einstufung „kein
Produktpfad“ ist keine Entwarnung für lokale Entwicklungsrechner: Installations-
und DevTools-Umgebungen bleiben betroffen, bis die Kette entfernt oder mit
geprüfter Fixversion ersetzt ist.

## Cargo-Features und Duplikate

| Bereich | Aktueller Featurebefund | Status | Maßnahme |
|---|---|---|---|
| reqwest Desktop | json, rustls-no-provider, system-proxy, blocking; blocking wird in ai/provider.rs:73,118 benötigt | VERIFIZIERT notwendig | behalten; nur bei Trennung in eigenen Client-Crates neu bewerten |
| reqwest Services | json, rustls, system-proxy; Plugin-Network/Registry verwenden async Client | VERIFIZIERT notwendig | behalten; redirects und Response-Limits sind Code-Sicherheitskontrollen |
| rustls | default-features=false, ring; Provider wird in updater/HTTP benötigt | VERIFIZIERT begrenzt | behalten; rustls-webpki patchen |
| wasmtime | default-features=false, cranelift, runtime; kein WASI-Feature im direkten Manifest | VERIFIZIERT minimiert, aber Cranelift groß | Sicherheitsfix zuerst; Compiler-/Runtime-Split nur nach Plugin-Produktentscheidung |
| zip | direkte Version 4.6.1 ohne default-features=false | UNGEKLÄRT | cargo tree -e features und zip-Dokumentation prüfen; keine Featureentfernung ohne Package-Test |
| printpdf | exakt 0.8.2, default-features=false, png/jpeg | VERIFIZIERT bewusst begrenzt | behalten; PDF-Specimens und lopdf-Fixkompatibilität prüfen |
| image | default-features=false, png/jpeg; profile.rs akzeptiert und normalisiert beide Formate | VERIFIZIERT notwendig | nicht auf ein Format reduzieren |
| rusqlite | bundled mehrfach direkt und über SeaORM-sync | VERIFIZIERT Architektur-/Portabilitätsentscheidung | erst nach SQLite-Kompatibilitätsaudit konsolidieren |
| sea-orm-sync | sync, macros, rusqlite, postgres-array, with-chrono/json/uuid, sqlite-use-returning-for-3_35 | VERIFIZIERT produktiv genutzt | Feature-by-feature testweise minimieren; kein Async-Rewrite |
| Tauri | tray-icon, image-png, macos-private-api; direkte native Features | VERIFIZIERT genutzt in tray/startup/macOS | macos-private-api nur separat auf Windows/Linux Target prüfen |
| Cargo Duplikate | mehrere Versionen von base64, rand_core, sha2, dirs, rusqlite, url, rustls; viele Tauri/SeaORM-Duplikate | VERIFIZIERT | nur direkte Constraints/Features harmonisieren; Upstream-Duplikate akzeptieren, wenn Parent-APIs sie erzwingen |

Open-Source-Alternativen: Für Wasm ist Wasmtime die bereits passende,
sandbox-orientierte Open-Source-Runtime; ein Wechsel zu Wasmer/wasmi würde die
Security- und ABI-Tests neu aufsetzen und bringt ohne Messung keinen Vorteil.
Für PDF bleibt printpdf durch die Repository-Regel die produktive Rust-Komposition;
genpdf oder pdf-writer wären Alternativen, aber kein belegter Drop-in-Ersatz für
die aktuelle PDF- und Font-Pipeline. Für Archive ist eine aktualisierte zip-
Linie der kleinste Fix; eine Eigenimplementierung ist nicht zu empfehlen.

## Performance- und Speicherbefunde

| Messpunkt | Beobachtung | Status | Konsequenz |
|---|---|---|---|
| node_modules | ca. 2.0 GB lokal, inklusive Plattform-/Tooling-Knoten | VERIFIZIERT | Dev-Tool-Kette trimmen; nicht als Bundle behaupten |
| target | ca. 68 GB lokal, offensichtlich Build-/Cache-Artefakte | VERIFIZIERT | Cleanup/Cache-Policy getrennt vom Dependency-Audit; nicht in Releasegröße einrechnen |
| apps/web/.next | ca. 262 MB lokal | VERIFIZIERT | Build-Cache/Dev-Output, kein gemessenes Release-Bundle |
| apps/web/out | ca. 4.6 MB lokal | VERIFIZIERT | statischer Exportindikator; routeweise JS-Analyse fehlt |
| pdfjs-dist | dynamischer Import plus eigener Worker in PdfViewer.tsx:17-21 | VERIFIZIERT | aktuelles Lazy-Loading beibehalten |
| Recharts | viele Wrapper und direkte Imports unter evilcharts | VERIFIZIERT | nicht initial sichtbare Chart-Slices messen/lazy laden |
| Wasmtime/Cranelift | 21 Wasmtime-Nodes/Featurepfade im Cargo-Duplikat-Output, großer native Compilerstack | VERIFIZIERT als Buildkosten-Indikator | keine Runtime-Entfernung ohne Plugin-Produktentscheidung; Release-Build messen |
| Sentry | @sentry/nextjs bringt mehrere Browser/Replay/OTel-Transitives | WAHRSCHEINLICH relevante Bundlekosten | traces/replay bereits stark begrenzt; client-only bleiben |
| Tiptap | viele Einzel-Extensions plus starter-kit | VERIFIZIERT | StarterKit-Extensionliste gegen explizite Extensions diffen |
| shadcn | CLI/Registry/MCP-Transitives liegen in Web-Dependencygraph | VERIFIZIERT | prüfen, ob Runtime-Dependency nötig ist oder CLI in devDependencies verschoben werden kann; nicht ohne Next/shadcn-Test |

## Sichere Entfernungskandidaten und Grenzen

Kein Kandidat erhält im aktuellen Tree die Einstufung sicher entfernbar.
Folgende Kandidaten sind nur WAHRSCHEINLICH entfernbar:

1. react-devtools: nur DEV_TOOLS.md und dev-Manifest; entfernt die Electron-/
   extract-zip-Kette. Akzeptanz: React-Debugging lokal dokumentiert, Bun-Install,
   bun audit und verify erfolgreich.
2. kbar: kein direkter Produktivimport; taskbar-Falsepositives ausgeschlossen.
   Akzeptanz: rg in Source/Generated/Plugin-Inputs, Next build und typecheck.
3. @tanstack/react-form, @tanstack/react-query, @tanstack/react-table, dice-ui,
   nuqs, zod, zustand: kein aktueller Produktivimport. Akzeptanz zusätzlich:
   kein package-export/registry-generator-Verbrauch und keine Contract-/Plugin-
   Tests, danach Lockfile-Reduktion und verify.
4. @next/bundle-analyzer und @biomejs/biome: keine aktive CLI-Konfiguration.
   Akzeptanz: DEV_TOOLS.md/Kommentare bereinigt oder bewusst als lokale Tools
   dokumentiert; keine CI-Erwartung.
5. Direkte cn-Version 0.5.0: lokales cn nutzt clsx/tailwind-merge; shadcn hat
   eigenes transitive cn 0.2.6. Akzeptanz: shadcn CLI erzeugt weiterhin gültige
   Komponenten und Typecheck bleibt grün.

## Priorisierte Maßnahmen mit Akzeptanzkriterien

| Priorität | Maßnahme | Nutzen | Aufwand | Risiko | Akzeptanzkriterien |
|---|---|---:|---:|---:|---|
| P0 | Wasmtime gepatchte kompatible Linie oder temporäre externe Wasm-Aktivierung blockieren | sehr hoch | hoch | hoch | cargo audit ohne wasmtime-Funde; Plugin-ABI-, Memory-, Fuel-, x86_64/arm64-Tests |
| P0 | rustls-webpki und tokio auf Fixstände evaluieren | hoch | mittel | mittel | cargo audit; API-, updater-, plugin-network- und server tests |
| P0 | zip Fixlinie evaluieren; manuelle Extraktion beibehalten/gegen Symlink-, duplicate-, zip-bomb Tests absichern | hoch | mittel | mittel | package tests, path traversal, symlink, file-count/size tests |
| P0 | lopdf/printpdf Kompatibilität prüfen | hoch | mittel-hoch | mittel | invoice PDF specimens, render tests, cargo audit |
| P1 | react-devtools als lokale Option statt Standard-DevDependency behandeln | hoch | niedrig | niedrig-mittel | devtools workflow explizit, Bun audit-Kette reduziert, verify |
| P1 | Root-JS-Kandidaten einzeln bereinigen | mittel-hoch | niedrig-mittel | mittel | keine Imports/Plugin-Contracts, lockfile diff nur erwartete Knoten, tsc/verify |
| P1 | Root/Web Ownership und Versionsdrift 0.9.38 vs 0.9.35 dokumentiert oder vereinheitlicht | mittel | niedrig | mittel | release metadata check und package-manager workspace resolution |
| P1 | Tiptap starter-kit/Einzel-Extensions konsolidieren | mittel | mittel | mittel | Notes editor typecheck/build plus feature smoke tests |
| P2 | shadcn, Sentry, Recharts, Shader, PDF.js routeweise messen | mittel | mittel | niedrig-mittel | before/after out artifact sizes and initial route chunk budget |
| P2 | Cargo direct constraints/features normalisieren | mittel | mittel-hoch | mittel | cargo tree -d reduction without target/test regressions |

## Audit-Verifikation

Frisch ausgeführt am 2026-10-24:

- bun pm ls --all: 741 Tree-Knoten.
- cargo metadata --format-version 1 --locked --no-deps: 11 Workspace-Crates.
- cargo audit --json: 28 Vulnerabilities, 0 unmaintained, 0 unsound.
- bun audit --json: Advisory-Objekte für cross-spawn, electron, extract-zip
  und got; die JSON-Struktur enthält 47 Electron-Einträge.
- cargo tree --workspace -d und cargo tree --workspace -e features: Duplikate
  und Featurepfade geprüft.
- Source-/Config-Suche: direkte JS-Imports, Rust-Crate-Imports,
  Plugin-Registry/Wasm/ZIP/TLS/Updater-Pfade geprüft.

Nicht ausgeführt: Dependencies ändern, installieren oder entfernen; cargo
build/check, bun build, native Tauri-Webview, Release-Bundleanalyse und
plattformübergreifende Runtime-Tests. Diese Grenzen bleiben bei allen
Performance- und Exploitability-Aussagen sichtbar.


