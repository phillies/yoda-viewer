# First-Run Class-Index Build Progress

> Source: next-features.md §2.8 · Effort: M
> Depends on: interacts with features/10 (rebuild endpoint — share the status machinery)

## Problem

First-run index build takes ~35 s on the example dataset (one label-file read per image,
sequential, inside `BackendState::from_settings` → `ClassIndex::load_or_build`,
`crates/yoda-web/src/lib.rs:100-109`). During that time the server hasn't even bound its
port — the browser can't connect at all, and there is no progress indication anywhere but
the log.

## Design — two changes, not one

### 1. Don't block startup on the index (structural fix)

Move `load_or_build` off the startup path:

- `BackendState.class_index` becomes `Arc<RwLock<ClassIndexState>>`:

```rust
enum ClassIndexState {
    Building { done: usize, total: usize },
    Ready(ClassIndex),
}
```

- `from_settings` initializes `Building { done: 0, total: flat_index.image_count }` and the
  router spawns the build (`spawn_blocking`, since it's sync fs work) after startup. The
  builder updates `done` every N (e.g. 256) images by briefly taking the write lock, and swaps
  in `Ready(index)` at the end.
- Refactor `load_or_build` to accept a progress callback:
  `load_or_build_with_progress(image_root, label_root, flat_index, on_progress: impl FnMut(usize, usize))`
  — keep the old signature delegating with a no-op callback (desktop + tests unchanged).
- Server binds immediately; tree and images work during the build.

### 2. Status endpoint + UI

- `GET /api/class-index/status` → `{ "state": "building", "done": 1234, "total": 8000 }`
  or `{ "state": "ready", "entry_count": 8000 }`.
- `GET /api/class-index` during build: return `202 Accepted` with the same status body
  (client treats as "retry later"), rather than blocking.
- UI: on startup, if status = building, show a slim progress bar in the filter-bar area
  (`Indexing classes… 1234 / 8000`) and poll status every 1 s until ready, then fetch
  `/api/class-index` and enable the filter chips. Poll with `spawn` + `TimeoutFuture` loop;
  stop on ready/unmount.
- Parallelize the build itself (optional, cheap win): the per-image work is independent
  file reads — chunk `flat_index` across `std::thread::scope` workers (4–8), merging maps at
  the end. Should cut the 35 s roughly by the parallelism factor on SSDs; keep sequential on
  first implementation if scope creep threatens.

## Interaction with saves during build

`save_labels` currently inserts into the index (lib.rs:479-486). During `Building`, apply the
insert to a small `pending: HashMap<…>` overlay stored in the `Building` variant, merged when
the build completes (last-writer-wins per path — the builder's scan of that file may race the
overlay; overlay wins since it reflects the in-memory latest save).

## Testing

- Axum test with a synthetic slow dataset (a few hundred generated label files): status
  endpoint transitions building→ready; `/api/class-index` 202 then 200; a save issued during
  build is present in the final index (overlay merge).
- UI: mock status sequence, assert progress bar renders then chips appear (or cover via E2E
  manual only — the polling loop is simple).

## Risks

- Lock contention from progress updates: update every 256 items, not per item.
- Desktop uses the same `build_router` → gets the async build for free; verify the webview UI
  poll path works identically.
