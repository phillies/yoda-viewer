# Move Blocking Filesystem I/O off the Async Runtime

> Source: optimizations-and-features.md §3.5 · Effort: M
> Depends on: nothing · Urgency: low for localhost, high before promoting shared-server use

## Problem

All Axum handlers in `crates/yoda-web/src/lib.rs` call synchronous repository methods
directly on the runtime worker:

- `image_bytes` → `fs::read` (multi-MB reads; lib.rs:429-447)
- `image_metadata` / `load_labels` → `image::image_dimensions` + `fs::read_to_string`
- `save_labels` → label write + index update
- `resolve_path` → `fs::canonicalize` per request

On local SSDs this is invisible. On NFS/CIFS-mounted training data (the realistic deployment),
one slow `fs::read` stalls a tokio worker; a burst of tree/image requests can stall them all,
freezing even `/api/health`.

## Design

Keep `DatasetRepository` synchronous (it's used from non-async contexts and its simplicity is
a feature). Add an async adapter at the web layer instead of asyncifying the trait:

```rust
// yoda-web
async fn blocking<T, F>(f: F) -> Result<T, ApiError>
where F: FnOnce() -> Result<T, ApiError> + Send + 'static, T: Send + 'static {
    tokio::task::spawn_blocking(f).await
        .map_err(|e| ApiError::internal(format!("blocking task panicked: {e}")))?
}
```

Handler pattern (state is `Arc<BackendState>` — clone into the closure):

```rust
async fn image_bytes(Extension(state): …, Query(query): …) -> Result<Response, ApiError> {
    let (bytes, content_type) = blocking(move || {
        let path = resolve_path(&state.image_root, &query.image_path)?;
        …existing body…
    }).await?;
    …
}
```

Apply to: `image_bytes`, `image_metadata`, `load_labels`, `save_labels`, `list_tree`,
`list_children`, and the fallback viewer builder (`build_fallback_view` walks the whole tree
recursively — the worst offender). `health`, `tree_status`, `tree_flat`, `class_map`,
`color_map`, `class_index_handler` are memory-only (after correctness/06 removes config disk
reads from the hot path — note `class_map()`/`color_map()` currently **re-read and re-parse
the YAML files on every request** via `load_class_map`/`load_color_map`; cache both in
`BackendState` at startup, which is also correct since the app treats them as static).

For `image_bytes` specifically, prefer `tokio::fs::read` (already-async, no thread hop) once
path resolution has run in `spawn_blocking`; or move to `tower-http` `ServeFile`-style
streaming as part of performance/06.

## Sizing

Default blocking pool is fine (max 512 threads). No config needed; do not add artificial
concurrency limits in v1.

## Testing

- Existing Axum handler tests pass unchanged (they drive the same public routes).
- Add a regression guard: handler that sleeps 200 ms inside `spawn_blocking` (test-only route
  or a unit test on `blocking`) while `/api/health` stays <10 ms — proves the runtime isn't
  starved. Practically: a `#[tokio::test(flavor = "multi_thread", worker_threads = 1)]` that
  fires a slow image request and a health request concurrently and asserts health completes
  first.

## Risks

- `spawn_blocking` closures need `'static`: minor `Arc`/`String` cloning of query params.
- Panic behavior changes from "unwind through handler" to `JoinError` — mapped to 500 above,
  which is an improvement.
