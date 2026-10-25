---
id: arch.plugins
title: Aevio Plugin Platform Architecture
category: architecture
domain: platform
status: experimental
source_paths:
  - crates/aevio-contracts/src/plugin.rs
  - crates/aevio-core/src/plugins.rs
  - crates/aevio-services/src/plugins.rs
  - crates/aevio-services/src/plugins_jobs.rs
  - crates/aevio-services/src/plugins_events.rs
  - crates/aevio-persistence/src/database/repositories/plugins.rs
  - crates/aevio-infrastructure/src/plugins.rs
  - packages/plugin-sdk/src/index.ts
  - apps/cli/src/main.rs
  - apps/desktop/tauri/src/commands.rs
  - apps/desktop/tauri/src/plugin_registry.rs
  - apps/desktop/tauri/src/plugin_network.rs
  - apps/web/src/features/plugin-store
  - apps/web/src/hooks/usePluginExtensions.ts
  - apps/api/src/app/views/plugins.rs
related_docs:
  - docs/security/PLUGIN_SECURITY.md
  - docs/architecture/server.md
  - docs/DATABASE_COMPATIBILITY.md
platforms: [macos, windows]
verification_commit: working-tree
---

# Plugin platform

The plugin platform is a host-owned extension boundary. A plugin contributes
versioned metadata, permissions and namespaced UI/command declarations; it
does not receive a database connection, arbitrary Tauri handles or direct
access to the React router.

## Runtime boundaries

`aevio-contracts::plugin` is the wire and manifest contract. It validates
manifest versions, semver compatibility, identifiers, package paths,
capability names and contribution namespaces. `aevio-core::plugins` owns the
legal lifecycle transitions. `aevio-services::plugins` owns catalog lookup,
permission grants, repeated `CapabilityGateway` checks and
install/enable/disable/uninstall orchestration. `@aevio/plugin-sdk` exposes a
versioned capability-specific TypeScript surface; it deliberately has no raw
IPC, Tauri or database escape hatch.

The SQLite adapter persists installations, operations, grants, audit events,
device-namespaced storage entries, runtime health, installation history, durable jobs and
  workspace composition preferences and job lease/retry state in additive schema
  versions 33 and 34 tables. The
desktop commands are the only local mutation boundary. Each successful
mutation emits `plugins-changed`; the webview subscribes through
`useTauriEvent` and reloads the catalog.

`plugin_contributions` composes only active installations. The host filters
commands, search providers, widgets and workflows against the corresponding
grants before returning the snapshot; the webview still maps navigation only
to an explicit host-route allow-list.

The runtime boundary exposes typed host operations for commands, search and
events. First-party `*/open` commands resolve only to known Aevio routes;
Project Planning and Music search contributions query their existing SeaORM
adapters. The Store's global search aggregates active providers with a hard
result bound and reports provider failures as partial results. Workspace
composition preferences persist visibility, pinning and order for declared
navigation/pages/widgets/settings contributions and can be restored to
defaults. Event
publication is capability-checked and emitted under a
`plugin-event:<plugin-id>:<event>` namespace. External WASM components are
validated at staging and execute through the bounded Wasmtime JSON ABI with no
WASI imports, a 16-MB memory limit, fuel budget and one allow-listed host
import. External HTML UI is served only from a plugin-specific
`aevio-plugin://` origin in a separate Tauri webview. The webview matches no
application capability, uses a restrictive CSP with no network connection, and
the host resolves only active staged files below the declared UI entrypoint
directory. Staging never executes native package bytes.

The component host ABI exposes only `plugin.info`, installation-scope namespaced storage, declared
jobs, validated event publication and native notification effects. Each method
re-enters the service capability gateway; the Tauri adapter performs the final
event emission or OS notification after the service returns validated effects.

Subscriptions require a host authorization call before the Tauri listener is
installed; undeclared events cannot be observed. Event publication is also
written to the local event log. A subscription may bind a declared event type
to a declared workflow action; those deliveries are persisted with a lease,
bounded retries and a dead-letter terminal state, then dispatched by the
existing desktop scheduler. The workflow still re-checks plugin state and
capabilities at execution time.

The SDK's Tauri transport is capability-shaped: storage, event publication and
subscription, durable jobs, native notifications and origin-approved network
requests map to dedicated commands and namespaced events. Network requests
allow only GET/POST, exact manifest-declared origins, bounded bodies, bounded
responses and no redirects. It never exposes a generic `invoke` function.
Dynamic command registration remains an explicit host error until a
serializable handler contract exists.

Jobs are persisted with an installation foreign key, bounded intervals and a
JSON payload. The offline scheduler atomically leases due jobs, emits
`plugin-job:<plugin-id>:<job-id>` with the payload, and advances the schedule
only after successful delivery. Failed delivery uses bounded exponential
backoff, clears expired leases on recovery, and moves a job to a durable
dead-letter state after three attempts. Disabling or quarantining the
installation prevents execution while preserving the pending schedule.

Workflow actions use the same bounded WASM host and persist an execution
record with a stable idempotency key, input/output snapshot, attempt count,
retry ceiling, cancellation state and dead-letter outcome. A repeated
successful key is read-only; a failed key can retry only below the bounded
attempt ceiling. Workflow action capabilities are checked again immediately
before execution.

The current first-party catalog uses host-mediated declarative contributions.
The webview only maps known contribution pages to existing host routes. Unknown
plugin pages are ignored, so a manifest cannot cause a dynamic import or route
escape. External HTML UI opens in the isolated webview described above and is
never loaded into the privileged webview.

Verified external ZIP packages are staged below the app data directory using
an atomic versioned rename. External updates preserve still-declared effective
permissions while newly requested capabilities remain explicit; removed
capabilities are never effective. Previous versions remain in history so
rollback restores it. Runtime failures are recorded
at the privileged command boundary; after three failures the host automatically
rolls back to a verified previous artifact when available, otherwise it
quarantines the plugin. Recovery returns a quarantined plugin to the installed
state without activating it. External installation also checks the desktop/API compatibility
range, requires installed required dependencies and rejects same-version or
downgrade updates.

First-party installation uses a versioned plan with a base installation
revision. The host revalidates the catalog release and rejects stale plans
after permissions or lifecycle state changed.

The authenticated HTTP server exposes `GET /api/v1/plugins` (with `q` and
`category` filters), `GET /api/v1/plugins/{plugin_id}`, signed publication at
`POST /api/v1/plugins/{plugin_id}/releases/{version}`, and content-addressed
artifact download at the matching `/artifact` endpoint. Registry metadata and
publisher keys are persisted through both the SQLite and PostgreSQL SeaORM
adapters. Uploads require an admin service credential and an active,
non-expired trusted publisher key; release versions are immutable, and
publisher/release revocation is fail-closed.

The desktop Plugin Store can connect to that registry with a user-supplied
Bearer token held only in the active webview state. The Tauri registry
transport bounds catalog and artifact responses, stages remote packages only
after the local digest/signature/manifest verifier succeeds, and then follows
the same permission and lifecycle path as a local package install. Disconnect
clears the token from the Store state; it is not persisted in Aevio settings
or the local database.

Every local capability evaluation is built from a host-owned execution context:
plugin and installation identity, release version, runtime instance, local
actor, installation workspace, device, owner role, and desktop platform. Grants
are then intersected with declared capabilities, resource scopes, expiration,
revocation, and the platform allow-list. The plugin never supplies or overrides
these identity fields.

The first-party Music module uses the existing `music_tracks` domain table.
Local audio is imported only through the native file picker, copied into
Aevio's data directory, and served through the bounded `aevio-music://track`
protocol after the host verifies the persisted track identity and path. Plugin
UI code never receives the original filesystem path and the host enforces a
100 MiB size limit plus an audio-extension allowlist.

For multi-device workspaces, the server also stores an explicit desired plugin
state per workspace and plugin. The state includes the requested release
version or update channel, enabled intent, requested permissions, and a
monotonic revision. Reading it requires workspace Viewer access; changing it
requires workspace Admin access. This is server intent only: it does not grant
local device permissions or replace local artifact verification and activation.
The desktop host exposes bounded authenticated transport for reading and
updating this state, while registry bearer tokens remain transient in the
active Store session.

Update policy is also local and offline-safe. Each plugin can select the stable,
beta or development channel, enable automatic-update intent, and pin an exact
semantic version. The registry update query selects the highest compatible
release and reports newly requested permissions before the user approves it.
The installer re-checks the policy at the privileged package boundary, so a
frontend selection cannot bypass a pin or channel restriction.

Automatic plugin updates run when the user explicitly connects a registry
session. Candidates with automatic updates enabled and no new permission
request are staged through the same verified installer; permission-changing
updates remain manual. Registry bearer tokens stay in transient UI/command
state and are never written to SQLite or plugin metadata.

The CLI exposes the same local RPC contract through `aevio plugins list`,
`plan`, `install`, `enable`, `disable`, `rm`, `search`, `execute`, `schedule`
and `cancel`; these operations also work with the app closed while preserving
the existing store lock and schema checks. Local developer tools additionally
validate manifests, pack deterministic unsigned archives, inspect packages and
verify signed packages. Unsigned archives are explicitly not production
installable.

`packages/plugin-sdk` publishes the versioned `@aevio/plugin-sdk` contract. It
provides typed manifest validation, declarative navigation/pages/widgets/
settings/service capability metadata, storage, events, notifications, network,
jobs and workflows, plus activation/deactivation hooks. Its Tauri adapter
exposes only capability-shaped host calls; there is no generic invoke or
database handle.

## Lifecycle and recovery

Install starts in `installed`, permission grants are explicit, and activation
requires every manifest-requested capability to be granted. Active plugins
must be disabled before uninstall. Uninstall removes staged runtime artifacts,
revokes grants, and keeps an `uninstalled` registration so plugin-owned user
data remains available for a later reinstall. Stored plugin data is deleted
only through the explicit UI action or `aevio plugins purge-data`. Every local
mutation records an operation and audit event. Failed or quarantined states
remain representable in the contract and persistence schema for recovery
tooling.

## Compatibility rule

Plugin changes must preserve existing SQLite files and offline behavior. Schema
changes are additive; opening the database never resets or synchronizes away
user data. Plugin storage includes installation, user, workspace and persistent
local-device identity; legacy rows migrate to a stable legacy-device namespace
without changing their values. New runtime capabilities belong behind the
service and Tauri boundaries rather than in page components.
