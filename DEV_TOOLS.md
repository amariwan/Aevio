# Dev-Tools Übersicht (Aevio)

## Rust-Tools (installiert via cargo/rustup)

### Core-Toolchain
| Tool | Befehl | Status |
|------|--------|--------|
| cargo | `cargo` | ✓ (Homebrew) |
| rustc | `rustc` | ✓ (Homebrew) |
| rustup | `rustup` | ✓ (neu installiert) |
| rustfmt | `cargo fmt` | ✓ (rustup component) |
| clippy | `cargo clippy` | ✓ (rustup component) |
| rust-analyzer | `rust-analyzer` | ✓ (rustup component) |

### Test & Qualität
| Tool | Befehl | Status |
|------|--------|--------|
| cargo-nextest | `cargo nextest run` | ✓ |
| cargo-llvm-cov | `cargo llvm-cov` | ✓ |
| cargo-audit | `cargo audit` | ✓ |
| cargo-deny | `cargo deny check` | ✓ |
| cargo-geiger | `cargo geiger` | ✓ |
| cargo-semver-checks | `cargo semver-checks` | ✓ |
| miri | `cargo +nightly miri run` | ✓ (nightly) |

### Entwicklung & Debugging
| Tool | Befehl | Status |
|------|--------|--------|
| cargo-watch | `cargo watch -x check` | ✓ |
| cargo-expand | `cargo expand` | ✓ |
| bacon | `bacon` | ✓ |
| flamegraph | `cargo flamegraph` | ✓ |
| cargo-fuzz | `cargo fuzz run` | ✓ |

### Analyse & Wartung
| Tool | Befehl | Status |
|------|--------|--------|
| cargo-machete | `cargo machete` | ✓ |
| cargo-bloat | `cargo bloat` | ✓ |
| cargo-hakari | `cargo hakari generate` | ✓ |
| cargo-udeps | `cargo udeps` | ⏳ (kompiliert noch) |
| cargo-edit | `cargo add/rm/upgrade` | ✓ (cargo-add, cargo-rm, cargo-upgrade) |

### Build & Release
| Tool | Befehl | Status |
|------|--------|--------|
| cargo-dist | `dist init/build` | ✓ (als `dist`) |
| cargo-release | `cargo release` | ✓ |

## Web/Node-Tools (installiert)

| Tool | Befehl | Status |
|------|--------|--------|
| vercel CLI | `vercel` | ✓ |
| playwright | `playwright` | ✓ |
| @next/bundle-analyzer | (npm devDep) | ✓ |
| react-devtools | `npx react-devtools` | ✓ |
| oxlint | `bun run verify` | ✓ (im Repo) |
| oxfmt | `bun run verify` | ✓ (im Repo) |
| tsc | `tsc --noEmit` | ✓ (im Repo) |
| shadcn CLI | `npx shadcn@latest add` | ✓ (npx) |

## Browser-Extensions (manuell installieren)

### React Developer Tools
- **Chrome:** https://chromewebstore.google.com/detail/react-developer-tools/fmkadmapgofadopljbjfkapdkoienihi
- **Firefox:** https://addons.mozilla.org/en-US/firefox/addon/react-devtools/
- **Edge:** https://microsoftedge.microsoft.com/addons/detail/react-developer-tools/gpphkfbcpidddadnkolkpfckpihlkkil
- **Standalone (CLI):** `npx react-devtools` (installiert als devDep)

### Next.js Dev Indicators
- Konfiguriert in `apps/web/next.config.ts` als `devIndicators.position: "bottom-right"`
- Zeigt Route-Status (Static/Dynamic) während der Entwicklung
- Dokumentation: https://nextjs.org/docs/app/api-reference/config/next-config-js/devIndicators

## IDE-Extensions (manuell installieren)

### VS Code / Cursor
1. **rust-analyzer** — Rust LSP
2. **Tailwind CSS IntelliSense** — Tailwind Klassen-Autocomplete
3. **Error Lens** — Inline-Fehleranzeige
4. **GitLens** — Git-Blame & History
5. **ES7+ React/Redux/React-Native snippets** — React Snippets
6. **Prettier** — Code-Formatierung
7. **ESLint** — Linting
8. **Even Better TOML** — TOML-Unterstützung (Cargo.toml)
9. **crates** — Cargo.toml Dependency-Versionen
10. **CodeLLDB** — Rust/C++ Debugger
11. **Tauri** — Tauri-spezifische Unterstützung
12. **Biome** — Alternative zu oxlint/oxfmt

### JetBrains (RustRover / IntelliJ)
1. **Rust Plugin** (eingebaut in RustRover)
2. **Tailwind CSS** Plugin
3. **Database Tools** (für SeaORM/SQLite)

### Neovim
1. **rust-analyzer** (via mason.nvim)
2. **tailwindcss-language-server**
3. **typescript-language-server**

## Schnellstart-Befehle

```bash
# Rust
cargo watch -x check          # Auto-Check bei Änderungen
cargo nextest run             # Schnelle Tests
cargo llvm-cov               # Coverage
cargo audit                  # Security-Audit
cargo deny check              # Dependency-Policy
cargo machete                # Ungenutzte Deps
cargo bloat                  # Binary-Größe
cargo geiger                 # unsafe-Nutzung
bacon                        # Background-Checks
cargo expand                 # Macro-Expansion
cargo flamegraph             # Profiling
cargo +nightly miri run      # UB-Detection

# Web
vercel dev                   # Vercel Dev-Server
vercel deploy                # Deploy
playwright test              # E2E-Tests
npx shadcn@latest add button # shadcn Component

# Repo
bun run verify               # Canonical Check
bun run fix                  # Auto-Fixes
just test                    # Alle Tests
```

## PATH-Konfiguration

Folgende Zeilen wurden zu `~/.zshrc` hinzugefügt:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Rustup-Env wird via `~/.cargo/env` geladen (automatisch von rustup installiert).
