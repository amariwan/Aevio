# Theme System — Design

**Date:** 2026-10-20
**Status:** Approved

## Intent

Adopt the [next-shadcn-dashboard-starter](https://github.com/amariwan/next-shadcn-dashboard-starter) theme model: each theme is a complete look (base surfaces + accent + categorical colors), and the user picks one palette with a separate light/dark variant toggle.

## Goals

- One `data-theme` attribute on `<html>` selects a complete token set (replaces the current two-bucket light/dark split)
- Each palette defines both a light and a dark variant
- A separate mode toggle (system / light / dark) picks the variant — exactly like the reference repo
- Theme picker UI in Settings > Appearance with palette preview tiles
- Preserve all existing semantic token names so component code does not change
- Preserve the `sharper` shape system and `--ui-scale` size modes

## Non-goals

- No new UI components beyond the theme picker tile
- No changes to Rust backend settings serialization format (stay JSON-compatible)
- No migration of existing user settings (a fresh `accent` value maps to a palette at read time)
- No animated theme transitions (instant swap, matching current behavior)

## Anti-goals

- Raw hex colors in component files (all colors flow through semantic tokens)
- A second, parallel theming system (the old `data-theme="light"` override block is removed)

---

## Current State

| File | Role |
|------|------|
| `apps/web/src/styles/app.css` | Single stylesheet: `:root` (dark default) + `:root[data-theme="light"]` override + `.dark` shadcn defaults + `@theme` token mapping |
| `apps/web/src/lib/settings.ts` | `Theme = "system" \| "light" \| "dark"`, `Accent = "green" \| "blue" \| "purple" \| "orange"`, `applyAppearance()` writes `data-theme`, `data-shape`, `data-size-mode`, and inline accent vars |
| `crates/aevio-core/src/settings/mod.rs` | Rust `Settings` struct with `theme: Theme`, `accent: Accent`, serialized to JSON, migrated from older formats, `appearance_init_script()` seeds `aevio.theme` + `aevio.shape` into `localStorage` |
| `apps/web/src/components/SettingsAppearance.tsx` (or similar) | Settings > Appearance page with theme radio + accent swatch row |

---

## Target Model

### Theme = Palette identity

```
data-theme="forest"        →  forest palette, light variant
data-theme="forest" + data-mode="dark"  →  forest palette, dark variant
```

The **palette** is the complete token set. The **mode** picks light/dark variant within that palette.

### Palette list

| Palette | Description | Derived from |
|---------|-------------|--------------|
| `zinc` | Neutral dark, the current default look | Current `:root` dark tokens |
| `forest` | Green accent | Current `green` accent + dark surfaces |
| `ocean` | Blue accent | Current `blue` accent + dark surfaces |
| `violet` | Purple accent | Current `purple` accent + dark surfaces |
| `sunset` | Orange accent | Current `orange` accent + dark surfaces |
| `mono` | True grayscale (no chroma in any token) | New |

Each palette defines:
- Background tokens (`--bg-canvas`, `--bg-rail`, `--bg-bar`, `--bg-panel`, `--bg-surface-1/2`, `--bg-inset`, `--bg-inset-soft`, `--bg-scrim`)
- Foreground tokens (`--fg-strong`, `--fg`, `--fg-muted`, `--fg-soft`, `--fg-faint`, `--fg-ghost`)
- Line tokens (`--line`, `--line-soft`, `--line-strong`)
- Accent tokens (`--accent`, `--accent-dim`, `--accent-soft`, `--accent-rgb`, `--accent-fg`)
- Canvas gradient tokens (`--glow-1`, `--glow-2`, `--canvas-1`, `--canvas-2`, `--canvas-3`)
- Settings card tokens (`--bg-settings-card`, `--settings-card-border`)
- Categorical palette (`--cat-1` through `--cat-12`)
- Decision tiers (`--tier-rule`, `--tier-personal`, `--tier-model`)
- Semantic shadcn tokens (`--background`, `--foreground`, `--card`, `--popover`, `--primary`, `--secondary`, `--muted`, `--accent`, `--destructive`, `--border`, `--input`, `--ring`, `--chart-1..5`, `--sidebar-*`)

### Mode = Variant selector

```
data-mode="light"  →  force light variant of the current palette
data-mode="dark"   →  force dark variant of the current palette
(data-mode omitted / "system")  →  follow prefers-color-scheme
```

This replaces the old `data-theme="light"` / `data-theme="dark"` selectors. The old `Theme` type `"system" | "light" | "dark"` becomes the `Mode` type.

### Accent removal

The `Accent` type and `ACCENTS` record are removed. The accent is now defined per-palette in CSS. `applyAppearance()` no longer sets inline accent vars — it reads `data-theme` + `data-mode` and lets CSS do the work.

---

## CSS Architecture

### File structure

```
apps/web/src/styles/
  app.css              ← @import "tailwindcss", @import "tw-animate-css", @import "./themes.css"
  themes.css           ← @import "./themes/zinc.css"; @import "./themes/forest.css"; ...
  themes/
    zinc.css           ← [data-theme="zinc"] light + [data-theme="zinc"][data-mode="dark"]
    forest.css         ← [data-theme="forest"] light + dark
    ocean.css
    violet.css
    sunset.css
    mono.css
```

### Theme file pattern (example: forest.css)

```css
[data-theme="forest"] {
  --bg-canvas: #1d201f;
  --bg-rail: #191c1b;
  /* ... all semantic tokens, light variant ... */
  --accent: #5bcf8f;
  --accent-dim: #48ab74;
  --accent-soft: rgba(91, 207, 143, 0.15);
  --accent-rgb: 91, 207, 143;
  --accent-fg: #10261a;
  --cat-1: #75a4e5;
  /* ... all categorical + shadcn tokens ... */
}

[data-theme="forest"][data-mode="dark"] {
  --bg-canvas: #f2f1ed;
  /* ... all tokens, dark variant ... */
}

@theme inline {
  --color-accent: var(--accent);
  /* ... same @theme mapping as today, repeated per file ... */
}
```

### Default (no data-theme set)

When no `data-theme` attribute is present, `:root` carries the `zinc` palette defaults (current dark tokens). This is the pre-paint fallback before JS runs.

### `data-mode` default

When `data-mode` is absent, the browser's `prefers-color-scheme` decides. CSS `@media (prefers-color-scheme: light)` overrides can be used inside each theme file as a fallback, but the JS `applyAppearance()` sets `data-mode` explicitly on load, so this is only a pre-paint concern.

---

## Settings UI

### Settings > Appearance

Replace the current theme radio + accent swatch row with:

1. **Palette grid** — 6 tiles (Zinc, Forest, Ocean, Violet, Sunset, Mono). Each tile shows:
   - Palette name
   - 4-5 color dots representing the palette's key colors
   - A checkmark or ring on the selected palette

2. **Mode toggle** — 3-way segmented control: System / Light / Dark

### State shape

```typescript
// Before
type Theme = "system" | "light" | "dark";
type Accent = "green" | "blue" | "purple" | "orange";
interface Settings { theme: Theme; accent: Accent; ... }

// After
type Palette = "zinc" | "forest" | "ocean" | "violet" | "sunset" | "mono";
type Mode = "system" | "light" | "dark";
interface Settings { palette: Palette; mode: Mode; ... }
```

### Migration

At read time (in the settings loader), map old settings to new:
- `theme: "dark"` → `palette: "zinc", mode: "dark"`
- `theme: "light"` → `palette: "zinc", mode: "light"`
- `theme: "system"` → `palette: "zinc", mode: "system"`
- `accent: "green"` → `palette: "forest"` (keep mode)
- `accent: "blue"` → `palette: "ocean"`
- `accent: "purple"` → `palette: "violet"`
- `accent: "orange"` → `palette: "sunset"`
- Missing `accent` → `palette: "zinc"`

This is a read-time migration in the TypeScript settings loader and the Rust `Settings::migrate()` path. The old `accent` field is kept in the Rust struct (ignored after migration) for backward compat.

---

## Rust Changes

### `crates/aevio-core/src/settings/mod.rs`

- Add `Palette` enum: `Zinc`, `Forest`, `Ocean`, `Violet`, `Sunset`, `Mono`
- Keep `Accent` enum but mark deprecated / remove from serialization
- `Settings` struct: replace `accent: Accent` with `palette: Palette`
- `Settings::migrate()`: when loading old JSON with `"accent"`, map to `palette`
- `appearance_init_script()`: seed `aevio.theme` (now palette name) + `aevio.mode` (light/dark/system) + `aevio.shape`

### `apps/web/src/lib/settings.ts`

- `applyAppearance()`:
  - Reads `settings.palette` → sets `data-theme`
  - Reads `settings.mode` → sets `data-mode` (or removes it for system)
  - Reads `settings.shape` → sets `data-shape` (unchanged)
  - Reads `settings.sizeMode` → sets `data-size-mode` (unchanged)
  - No longer sets inline accent vars
- Export `PALETTES: Record<Palette, { label: string; colors: string[] }>` for the UI tiles

---

## Pre-paint Flash Prevention

The `appearance_init_script` in Rust seeds `localStorage` with the saved palette + mode. The inline `<script>` in `index.html` reads these and sets `data-theme` + `data-mode` on `<html>` before first paint. Update the script to handle the new key names.

---

## Verification

1. `bun run verify` passes (oxlint + oxfmt + tsc + cargo fmt + cargo clippy)
2. `bun run dev:screenshot` shows the app rendering correctly with each palette
3. Settings > Appearance: clicking a palette tile immediately repaints the app
4. Mode toggle switches light/dark without reload
5. Shape (rounded/sharper) and size mode still work with every palette
6. Old settings JSON with `accent` loads without error and maps to the right palette
