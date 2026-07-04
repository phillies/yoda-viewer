# Single-Binary Distribution + `yoda` CLI

> Source: optimizations-and-features.md §7.8 · Effort: M — the highest-leverage adoption item
> Depends on: infra/01 (CI builds artifacts) · Eliminates: the entire SSR-fallback confusion
> class (see infra/02 — coordinate; this makes the "no assets" state impossible)

## Concept

```bash
cargo install yoda-viewer          # or brew / release binary
yoda serve --images ./images --labels ./labels --classes data.yaml --open
```

One binary, WASM bundle embedded, no asset directories, no env-var incantations. CLI args as
the friendly front door; env vars remain for Docker.

## Implementation

### 1. Embed the web bundle

- Build pipeline: `dx build --release` runs **before** `cargo build` and its `public/`
  output is included via `include_dir!` (`include_dir` crate) or `rust-embed`
  (prefer `rust-embed`: built-in mime guessing, debug-mode reads from disk for fast
  iteration — exactly the dev/prod split we want).
- Ordering problem: cargo can't easily run dx first. Options:
  a. **CI/release-only embedding** (recommended): a cargo feature `embed-ui`; the release
     workflow runs dx, then `cargo build --features embed-ui` with
     `YODA_UI_DIST=<path>` consumed by `rust-embed`'s `#[folder = env!(…)]`. Local dev keeps
     today's flow.
  b. `build.rs` invoking dx — fragile (network, tool presence); avoid.
- Serving: when `embed-ui` is active, replace the `has_client_assets` disk check
  (`crates/yoda-web/src/lib.rs:330-363`) with an embedded-asset router (serve
  `index.html` at `/`, hashed assets with `Cache-Control: immutable`). Hydration parity:
  the Dioxus fullstack `serve_dioxus_application` path expects specific asset layout —
  verify SSR+hydration works from memory, else serve the embedded bundle as a static SPA
  (CSR-only) — acceptable: the app is client-driven anyway (all data via `/api`).
  **This is the main technical unknown; prototype it first.**

### 2. CLI (`clap` derive)

Rename `yoda-web`'s binary or add a `yoda` bin target:

```
yoda serve   --images <dir> --labels <dir> [--classes <yaml>] [--colors <yaml>]
             [--host 127.0.0.1] [--port 8080] [--read-only] [--open]
yoda check   … validate dataset (paths, orphans, parse errors — reuses features/08 + correctness/01 machinery)
yoda convert / export        (bold/06 lands here)
yoda --version / --help
```

- Precedence: CLI args > env vars > defaults — implement as
  `YoDaSettingsOverrides` (already exists, `crates/yoda-config/src/lib.rs:33-41`) applied
  over `from_env` (the plumbing was built for exactly this ✓).
- `--open`: launch the browser (`open` crate) after bind.
- No subcommand + no config → print friendly quickstart help, not a panic.

### 3. Distribution

- Release workflow (infra/01 extension): matrix build (linux-x86_64/musl for portability,
  windows-msvc, macos-aarch64+x86_64) with `embed-ui`; upload to GitHub Releases; version
  from workspace.
- crates.io: publishing with embedded UI requires the dist files in the package —
  `include` them in the crate (package size limit 10 MB; dx wasm bundles ~2–4 MB gzipped —
  measure; if too big, crates.io install builds without `embed-ui` and prints the dx
  instruction, while release binaries carry the UI. Decide from real numbers).
- Homebrew tap (`phillies/homebrew-yoda`) pointing at release binaries — 30 lines of formula.

## Testing

- CI job: build with `embed-ui`, run the binary against the E2E fixture,
  assert `/` serves the app (infra/03's spec runs against this artifact — best possible
  integration test).
- CLI: `assert_cmd` tests for precedence (env vs flag), `check` subcommand outputs.

## Risks

- Dioxus fullstack vs static-embed hydration mismatch (flagged above) — prototype before
  committing to the CSR fallback or SSR-from-memory path.
- Binary size (ORT-free base ~20–30 MB with assets — fine). Keep bold/01's `infer` feature
  out of the default artifact.
