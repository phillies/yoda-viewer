# CI Pipeline (GitHub Actions)

> Source: optimizations-and-features.md §4 + next-features.md §3.6 · Effort: S
> Depends on: nothing · Do this before merging anything else in these docs.

## Problem

`.github/` contains only an agent definition — no workflows. `cargo fmt`, `clippy -D warnings`,
and the test suite are documented in the README but nothing enforces them on PRs (history shows
Copilot-authored PRs where this matters). The `dx build` path and Dockerfile are similarly
unexercised, which is how "SSR fallback by default" surprises keep happening.

## Design

`.github/workflows/ci.yml`, three jobs:

```yaml
name: CI
on:
  push: { branches: [main] }
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable   # rust-toolchain.toml pins the actual version
        with: { components: "rustfmt, clippy" }
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --workspace

  web-build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { targets: wasm32-unknown-unknown }
      - uses: Swatinem/rust-cache@v2
      - run: cargo install dioxus-cli --version 0.7.2 --locked   # keep in sync with Dockerfile ARG
      - run: cd crates/yoda-web && dx build --release
      - run: cargo build --release -p yoda-web --features server

  docker:
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'    # push-to-main can publish later
    steps:
      - uses: actions/checkout@v4
      - uses: docker/setup-buildx-action@v3
      - uses: docker/build-push-action@v6
        with: { push: false, cache-from: "type=gha", cache-to: "type=gha,mode=max" }
```

Notes:

- `rust-toolchain.toml` exists in-repo; `dtolnay/rust-toolchain` respects it — the `@stable`
  ref is just the action version channel. Verify the pinned toolchain (1.85+, edition 2024).
- `dioxus-cli` install dominates `web-build` time on cold cache; `Swatinem/rust-cache` caches
  `~/.cargo` binaries — confirm it caches installed bins (it caches registry + target; for the
  binary use `cargo-bins/cargo-binstall` or `taiki-e/install-action@v2` with
  `tool: dioxus-cli@0.7.2` for a prebuilt fetch, ~seconds).
- Docker job uses GHA layer cache; pairs with performance/08's cache-friendly Dockerfile.

## Dependency-pin watchdog (`time = "=0.3.51"`)

The workspace pins `time` due to a cookie/time API mismatch (`Cargo.toml:53-54`). Add a
scheduled job (weekly cron) that runs `cargo update --dry-run` and
`cargo update -p time --dry-run` and opens/refreshes an issue when the pin blocks newer
versions — or simpler: a `# TODO(remove-pin)` check via `cargo deny`/`cargo outdated` step
with `continue-on-error: true` so it's visible but non-blocking. Minimum viable: a comment in
Cargo.toml linking a tracking issue + `cargo outdated` in the weekly job.

## Branch protection

After the workflow is green on main: require `check` + `web-build` for PR merge. Keep `docker`
non-required initially (flakier, slower).

## Follow-ups enabled by this

- infra/03 (E2E smoke) plugs in as a fourth job consuming `web-build` artifacts.
- Release automation (tag → docker publish + binary artifacts) — see bold/08
  (single-binary CLI) for the artifact set worth publishing.

## Risks

- `dx build` may need system packages on the runner (openssl headers) — mirror the
  Dockerfile's `apt-get install build-essential pkg-config libssl-dev` if it fails.
- CI minutes: full cold build ~10–15 min; caches bring PR iterations to ~2–4 min.
