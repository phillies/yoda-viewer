# Fix Per-Edit O(dataset) Class-Index Writes

> Source: optimizations-and-features.md §3.4 · Effort: S (debounce) / M (incremental store)
> Depends on: correctness/02 (atomic writes) · Superseded eventually by: features/11 (SQLite)

## Problem

Every `PUT /api/labels` (`crates/yoda-web/src/lib.rs:479-486`):

```rust
if let Ok(mut index) = state.class_index.write() {
    index.entries.insert(image_rel, new_class_ids);
    let cache_path = …;
    let _ = index.save_to_disk(&cache_path);   // serializes ALL entries, every edit
}
```

- `save_to_disk` serializes and rewrites the **entire** index (one entry per dataset image) on
  every single label edit — hundreds of ms of JSON + I/O on large datasets, per dropdown change.
- The disk write happens **while holding the `RwLock` write guard**, blocking every concurrent
  `GET /api/class-index` (and other saves) for the duration.
- `let _ =` swallows write failures silently.

## Design (two stages)

### Stage 1 — dirty flag + debounced flush (small, do now)

1. In-memory update stays synchronous and correct (the `insert` above).
2. Replace the inline save with marking dirty:

```rust
struct BackendState {
    …,
    class_index: Arc<RwLock<ClassIndex>>,
    class_index_dirty: Arc<AtomicBool>,
}
```

3. A flusher task spawned in `BackendState::from_settings`' caller (needs a tokio context —
   spawn it in `build_router` since that's always called from async main / the desktop runtime
   thread; guard with `tokio::runtime::Handle::try_current`):

```rust
loop {
    tokio::time::sleep(Duration::from_secs(5)).await;
    if dirty.swap(false, Ordering::AcqRel) {
        let snapshot = { index.read().unwrap_or_poisoned().clone() }; // clone under read lock
        tokio::task::spawn_blocking(move || snapshot.save_to_disk(&cache_path)).await;
        // on error: log via tracing::warn! and re-set dirty
    }
}
```

   Cloning under a **read** lock and writing outside any lock fixes the blocking problem.
4. Flush on shutdown: `yoda-web/src/main.rs` already enables the tokio `signal` feature —
   wrap `axum::serve(...).with_graceful_shutdown(ctrl_c_and_flush)` and flush there; the
   desktop path flushes when its runtime future ends (add the same graceful-shutdown wiring).
5. Failure logging: replace remaining `let _ = save_to_disk` in `load_or_build`
   (`class_index.rs:73`) with `tracing::warn!` on error.

Worst case after Stage 1: 5 s of index staleness on crash — the load path already
self-heals (missing entries get rescanned; stale entries only mislead the filter until the
next edit of that image), acceptable for a cache.

### Stage 2 — incremental store

If datasets grow to the point where even the 5 s snapshot serialize is heavy (>~500k entries),
move the cache to `redb` or SQLite with one row per image (`path TEXT PRIMARY KEY,
class_ids BLOB`) and point-writes per save. This folds into features/11 (SQLite metadata
store) — don't build it standalone unless 11 is rejected.

## Testing

- Unit: dirty flag set by save handler; flusher writes once for N rapid edits (use
  `tokio::time::pause` + `advance`).
- Existing Axum save test still passes (in-memory index visible immediately via
  `GET /api/class-index` even before flush — add an assertion for that read-your-writes
  property).

## Risks

- Poisoned-lock handling: current code uses `if let Ok` (skips update on poison). Keep the
  lenient stance but log it; a poisoned index lock should never take down saves of the actual
  label file (which happens before the index update — verify ordering stays that way).
