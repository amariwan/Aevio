---
id: security.plugins
title: Aevio Plugin Security Model
category: engineering
domain: platform
status: experimental
source_paths:
  - crates/aevio-contracts/src/plugin.rs
  - crates/aevio-infrastructure/src/plugins.rs
  - crates/aevio-services/src/plugins.rs
  - crates/aevio-services/src/plugins_events.rs
  - packages/plugin-sdk/src/index.ts
  - apps/desktop/tauri/src/commands.rs
  - apps/desktop/tauri/src/plugin_network.rs
related_docs:
  - docs/architecture/plugins.md
  - docs/security/SECURITY_MODEL.md
platforms: [macos, windows]
verification_commit: working-tree
---

# Plugin security model

Plugins are untrusted input until the host validates their manifest and
release. The trust boundary is the desktop host: plugins never receive raw
SQLite/SeaORM objects, Tauri `AppHandle`, filesystem roots or unrestricted
network access.

## Verification requirements

`aevio-infrastructure::plugins::verify_artifact` enforces the maximum artifact
size, validates the manifest, compares the SHA-256 digest in constant time,
decodes the publisher public key and verifies the Ed25519 release signature.
Package paths reject absolute paths, parent traversal and empty components.
Publisher identity and signature metadata are carried in the shared contract.
The package reader additionally bounds file count and unpacked size, rejects
duplicate paths, and stages only through an atomic versioned rename.
Manifest deserialization rejects unknown fields, validates declared assets,
dependencies, event subscriptions and every capability-bearing contribution.

The first-party built-ins are declarative host modules and therefore have no
external artifact. They are still permission-gated and audited. Registry
uploads require an admin service credential, an active trusted publisher key,
and the same verifier before the content-addressed artifact is persisted.
Digest or signature failure never reaches the registry or activation.

## Capability policy

Capabilities are an explicit allow-list in the contracts crate. A manifest can
request capabilities but cannot grant them. The user grants only requested
capabilities, the service rejects unknown grants, and `CapabilityGateway`
re-checks every requested capability at activation. Effective grants are also
intersected with the current manifest, so an update or rollback cannot retain
a capability the active manifest no longer declares. Database capabilities are
deliberately absent; plugin storage is namespaced and mediated by the host.
The TypeScript SDK exposes only capability-specific interfaces and must never
be treated as the authority: the host repeats these checks.

The network capability requires an exact HTTP(S) origin in the manifest's
permission scope and an explicit grant. The Tauri boundary rejects credentials,
unsupported methods, redirects, oversized request bodies and oversized or
non-text responses before returning data to the plugin.

The desktop notification adapter also re-checks `notifications:show` in the
service layer before using the native notification plugin. Titles and bodies
are bounded and control characters are rejected; notification requests are
audited without storing their text.

Runtime commands, search and event publication re-check active state and grants
at the host boundary. External WASM commands use the bounded JSON ABI and a
Wasmtime engine with no WASI imports, a 16-MB linear-memory limit and a fuel
budget. The only host import is the capability-gated `aevio_host.call`; unknown
imports and malformed exports are rejected. Unknown command handlers are
rejected instead of being interpreted as executable code.

Event subscriptions are authorized before the frontend listener is created:
the plugin must be active, hold `events:subscribe`, and declare the exact
event type in its manifest. Published payloads are bounded and persisted
before delivery. Workflow deliveries are separately capability-checked,
leased, retried at most three times, and dead-lettered after exhaustion;
event publication never grants the subscriber additional permissions.

Job scheduling requires `jobs:schedule`, a declared namespaced job, a bounded
interval and a bounded JSON payload. The offline scheduler only emits a
namespaced host event after re-checking that the installation is active.

## UI and operational safety

Dynamic navigation is allow-listed against existing host routes. Unknown pages
are not imported. Store actions use the Tauri command boundary and refresh from
Rust after each mutation. Installations, operations and permission changes are
audited. An active plugin cannot be removed without an explicit disable step.

The registry exposes authenticated metadata and artifact download endpoints,
stores immutable releases by SHA-256 digest, supports publisher-key rotation
with expiry/revocation, and supports release revocation. Third-party
HTML UI is served from a plugin-specific custom origin in a separate webview
with no matching Tauri capability, no network CSP permission, path confinement,
and `nosniff`/no-store response headers. Native plugin binaries remain
forbidden. External WASM is executable only through the bounded ABI; it is
never loaded into the privileged webview.
