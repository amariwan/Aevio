---
id: docs.i18n.audit
title: Aevio i18n Audit
category: architecture
domain: platform
status: implemented
source_paths:
  - apps/web/src/lib/i18n.ts
  - apps/web/src/hooks/useI18n.tsx
  - apps/desktop/tauri/src/i18n.rs
  - scripts/i18n-check.mjs
  - scripts/i18n-check.test.mjs
related_docs:
  - docs/i18n/internationalization.md
  - docs/i18n/locale-resolution.md
  - docs/i18n/translation-catalog.md
  - docs/i18n/testing.md
platforms:
  - macos
  - windows
verification_commit: working-tree
---

# Aevio i18n Audit

## 1. Analyzed directories and files

- Root project rules: `AGENTS.md`, `package.json`, `turbo.json`, `justfile`
- Frontend: `apps/web/src/**` (App Router under `app/[lang]`, `features/**`, `components/**`, `shell/**`, `hooks/**`, `lib/**`)
- Shared packages: `packages/**`
- Desktop/Tauri: `apps/desktop/tauri/src/**` (including `i18n.rs`, `coach/**`, `breaks/**`, `widget.rs`, `pulse.rs`, `summary.rs`)
- Rust crates: `apps/api`, `apps/cli`, `crates/**`
- Scripts and tests: `scripts/*.mjs`, `scripts/*.test.mjs`
- Documentation: `docs/**`

Total frontend source files scanned: all `.ts`/`.tsx` under `apps/web/src`.

## 2. Found i18n systems

- **Frontend (Next.js)**: single catalog in `apps/web/src/lib/i18n.ts` with `en` and `de` message maps, `translate()`, `resolveLanguage()`, `localeFor()`, pluralization helper. Consumed via `useI18n()` hook (`apps/web/src/hooks/useI18n.tsx`) and `[lang]` URL routing.
- **Rust/Tauri**: `apps/desktop/tauri/src/i18n.rs` with `ResolvedLanguage`, `resolve()`, `from_settings()`, and localized helper functions for tray, coach, breaks, tooltips.
- **Settings**: language preference persisted in Rust settings (`Language::De/En/System`) and mirrored to frontend via app shell.

No second parallel translation logic was introduced.

## 3. Hardcoded texts

- Initial scan found 183 unique hardcoded JSX text nodes in frontend components.
- After this audit's conversions, 110 unique hardcoded JSX text nodes remain (mostly technical labels, brand strings, and option values).
- Rust-side hardcoded user-facing strings remain in a few window titles and intervention defaults (documented in remaining issues).

## 4. Missing and unused translation keys

- Before audit: `en` had 610 keys, `de` had 567 keys -> 43 keys missing in German. All 43 were added to `de`.
- After audit: 1047 keys in both `en` and `de` (parity: 0 missing, 0 extra).
- Static scan reports ~102 potentially unused keys, many of which are used dynamically (e.g. via `COLUMN_LABEL_KEY`, nav groupings, dynamic `dashboard.${metric}` templates). These are documented as false positives.
- Used-but-undefined keys: 0.

## 5. Changes performed

- Added 43 missing German translations for placeholder/example/input keys.
- Converted hardcoded UI text to i18n in:
  - `features/coach/**` (Settings + 9 components, ~188 new keys)
  - `features/focus/Focus.tsx`, `features/finance/Finance.tsx`, `features/dashboard/Dashboard.tsx`, `DashboardOverview.tsx`
  - `features/calendar/calendar/TimelineSettingsDialog.tsx`, `TimelineSettingsMenu.tsx`
  - `features/plugin-store/PluginWorkspaceStatePanel.tsx`, `PluginUpdatePolicyPanel.tsx`
  - `features/staffing/*`
- Added `scripts/i18n-check.mjs` automated check (key parity, duplicate detection, interpolation placeholders, used/unused keys, hardcoded JSX text count).
- Added `scripts/i18n-check.test.mjs` and integrated into `node --test scripts/*.test.mjs`.
- Fixed type errors in `shell/route-url.ts`, `app/[lang]/settings/page.tsx`, `features/dashboard/useDashboardView.ts` to keep `verify` green.
- Total keys: 610 -> 1047 in both languages.

## 6. Test results

- `bun run verify`: passed (oxlint, oxfmt, tsc, docs metadata, cargo fmt, clippy).
- `node --test scripts/*.test.mjs`: 33 tests passed, 0 failed.
- `cargo test --manifest-path apps/desktop/tauri/Cargo.toml`: 81 tests passed, 0 failed.
- `bun scripts/i18n-check.mjs`: passed (no blocking errors; warnings only for potentially unused/dynamic keys and remaining hardcoded text nodes).

## 7. Remaining issues

- 110 unique hardcoded JSX text nodes remain (technical labels, brand strings, enum option values, and a few component-level strings). These are tracked by `i18n-check.mjs` and can be migrated incrementally.
- Rust-side hardcoded strings remain in window titles (`"Aevio Timer"`, `"Aevio Productivity Coach"`) and coach intervention defaults (German strings in `adaptive_commands.rs`). These are partially localized via `i18n.rs` but not all paths.
- Automated check uses heuristics; dynamic key usage (e.g. `dashboard.${metric}`) requires manual review to avoid false positives.
- Swift sidecar strings were not fully audited in this pass.
