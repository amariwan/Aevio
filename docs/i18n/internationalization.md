# Internationalization Architecture

## Source of Truth

- Frontend locale catalog: `apps/web/src/lib/i18n.ts` (`messages.en`, `messages.de`).
- React integration: `apps/web/src/hooks/useI18n.tsx` (`I18nProvider`, `useI18n`).
- Settings persistence: `apps/web/src/lib/settings.ts` (`language: system | de | en`), backed by Rust `aevio_core::settings`.
- Rust native copy: `apps/desktop/tauri/src/i18n.rs`.

## Locale Resolution

Priority: explicit selection → system/browser language (when `system`) → English fallback.
Variants normalized: `de-*` → `de`, `en-*` → `en`, unknown → `en`.

## React Integration

`I18nProvider` reads settings, resolves language, sets `document.documentElement.lang`, exposes `t(key, values)`. Language switch is reactive without restart.

## Rust/Tauri Integration

`i18n::from_settings` resolves language including OS env vars for `system`. Native notifications, tray menu/tooltip use localized copy. IPC errors use structured codes.

## Formatting

Centralized in `apps/web/src/lib/format.ts` using resolved locale. Language, region, timezone, currency are distinct concepts; switching language does not convert amounts or change currency/timezone.

## Translation Guidelines

- Keys namespaced by feature (`nav.`, `break.`, `pulse.`, ...).
- Interpolation via `{name}` placeholders.
- Pluralization via `_one`/`_other` suffix keys.
- Technical logs remain English.

## Adding a Language

1. Add variant to `ResolvedLanguage` and `Language` enums (TS + Rust).
2. Add catalog object in `messages`.
3. Add Rust native copy.
4. Update resolver + tests + docs.

## Adding a Translation

Add key to both `en` and `de` catalogs. Typecheck enforces key validity via `MessageKey`.

## Hardcoded String Audit

Run `grep`/oxlint as documented. Remaining hardcoded UI strings are tracked; migration in progress.

## Known Exceptions

- Product/brand names, user content, AI-generated content, external system content remain unlocalized.
