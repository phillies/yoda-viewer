# Docker Build: Layer Caching, Pinning, Non-Root Runtime

> Source: optimizations-and-features.md §3.8 · Effort: S
> Depends on: nothing · Related: infra/01 (CI) should build this image

## Problem

`Dockerfile` currently:

1. `cargo install dioxus-cli --locked` — unpinned version; recompiles on every base-image
   change; several minutes.
2. `COPY . .` **before** any build → any source edit invalidates everything after, so every
   build recompiles all dependencies plus the workspace from scratch.
3. Runtime image runs as root; no healthcheck; `.dockerignore` coverage unverified against
   `target/` and `example_data/`.

## Design

### 1. Pin and cache the toolchain

```dockerfile
ARG DIOXUS_CLI_VERSION=0.7.2   # keep aligned with dioxus workspace dep
RUN cargo install dioxus-cli --version ${DIOXUS_CLI_VERSION} --locked
```

Better: use `cargo binstall` or the prebuilt `dx` release binary (download + checksum) to cut
~5 min of compile; acceptable to defer.

### 2. Dependency layer caching — two options

**Option A: cargo-chef (recommended)**

```dockerfile
FROM rust:1-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt-get update && apt-get install -y build-essential pkg-config libssl-dev
ARG DIOXUS_CLI_VERSION=0.7.2
RUN cargo install dioxus-cli --version ${DIOXUS_CLI_VERSION} --locked
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json          # deps only, cached
COPY . .
RUN cd crates/yoda-web && dx build --release
RUN cargo build --release -p yoda-web --features server
```

Note: `dx build` compiles the wasm target; `cargo chef cook` covers the native profile only.
Add a second cook for the wasm target
(`cargo chef cook --release --target wasm32-unknown-unknown -p yoda-web`) if measurements show
the wasm dep build dominating; requires `rustup target add wasm32-unknown-unknown` in the
builder (the `rust-toolchain.toml` may already pull it — verify).

**Option B: BuildKit cache mounts** (simpler file, requires BuildKit — fine for CI + modern
Docker): keep the current structure, add
`RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/app/target …`
to the build steps. Choose A if builds must be cache-effective on plain `docker build` without
BuildKit assumptions; B is less invasive.

### 3. Runtime hardening

```dockerfile
FROM debian:trixie-slim AS runtime
RUN useradd --system --uid 10001 yoda
USER yoda
HEALTHCHECK --interval=30s CMD ["/usr/local/yoda-web/yoda-web-healthcheck"]  # or curl-less: see below
```

- No curl in slim images: simplest healthcheck is a tiny `--health-cmd` using the binary
  itself — add a `yoda-web --healthcheck` flag that GETs `/api/health` via reqwest and exits
  0/1 (5 lines in `main.rs`), or skip HEALTHCHECK in v1.
- Data mounts (`/data/images`, `/data/labels`) must be readable by uid 10001 — document in
  README (`docker run --user` override escape hatch). **Note:** the class-index cache writes
  to the *label* dir; read-only label mounts break it → ties into features/12 (read-only mode)
  which should also relocate/disable the cache write.

### 4. Housekeeping

- Extend `.dockerignore`: `target/`, `dist/`, `example_data/`, `.git/`, `docs/`.
- `apt-get` cleanup (`rm -rf /var/lib/apt/lists/*`) in builder.

## Acceptance / measurement

- Second build after touching one `.rs` file: dependency layers all `CACHED`; total rebuild
  time cut from ~full to ≈ workspace-compile-only. Record before/after times in the PR.
- `docker run` as non-root serves the example dataset.

## Risks

- cargo-chef recipe drift with the `edition = "2024"` workspace — chef supports it in current
  releases; pin `cargo-chef` version.
- Pinned `dx` version must track the `dioxus` crate minor version; add a comment linking the
  two locations (Cargo.toml workspace dep + Dockerfile ARG).
