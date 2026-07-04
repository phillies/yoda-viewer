# Class-Index Refresh: Rebuild Endpoint + Filesystem Watcher

> Source: next-features.md §1.5, §3.4 + optimizations-and-features.md §5.4 · Effort: S + M
> Depends on: features/07 (build-status machinery — do that first, this reuses it)

## Problem

`.yoda_class_index.json` is built at startup and patched only via the app's own saves. Labels
edited by external tools (training scripts, other annotators, `sed`) drift silently
(next-features §1.5). Restarting the server is the only recovery.

## Stage 1 — Manual rebuild endpoint (an afternoon)

- `POST /api/class-index/rebuild` → flips state to `Building` (features/07's
  `ClassIndexState`), spawns `load_or_build_with_progress` **bypassing the disk cache**
  (add a `force: bool` — skip `load_from_disk`, rescan everything), returns `202` with the
  status body. Idempotent: a rebuild while building returns the current status.
- Rescan also refreshes the *flat index*? No — keep scope tight: images added/removed require
  a tree rescan too, which is a different lock (`flat_index` is currently an immutable
  `Arc<FlatIndex>` in `BackendState`, `crates/yoda-web/src/lib.rs:88`). Make `flat_index`
  an `Arc<RwLock<Arc<FlatIndex>>>` in this change and rescan both — it's the same walk and
  users who edited labels externally very often added images too. Handlers clone the inner
  `Arc` per request (cheap).
- UI: a small refresh button (`⟳`) in the filter-bar header → POST, then reuse features/07's
  polling progress UI; when done, re-fetch `/api/tree/flat` + `/api/class-index`.

## Stage 2 — `notify` watcher (auto-freshness)

- Dependency: `notify = "8"` (`RecommendedWatcher`, debounced via `notify-debouncer-mini`).
- Watch `label_root` recursively. On debounced events (500 ms window):
  - `*.txt` modified/created → re-run `extract_class_ids_from_label_file` for the mapped
    image path, patch the in-memory index, mark dirty (performance/04 flusher persists).
  - `*.txt` removed → set entry to `[]`.
  - Ignore `.yoda_class_index.json` self-writes (path match) — otherwise the flusher triggers
    the watcher in a loop.
- Watch `image_root` too? Defer: image add/remove → require manual rebuild (Stage 1 button),
  noted in the response of a new `GET /api/dataset-info`. Watching the image tree correctly
  (folder moves, bulk copies) is where watchers get gnarly; don't buy it until asked.
- **Client freshness**: patching the server-side index doesn't update connected clients. V1:
  none (client refreshes on next full load; the refresh button exists). V2: SSE endpoint
  `/api/events` broadcasting `{"type":"class-index-changed"}`; client re-fetches on message.
  SSE via axum's `Sse` response is ~40 lines; worth it only alongside bold/04
  (multi-user) — mark as such.
- Lifecycle: watcher owned by `BackendState`; keep the `RecommendedWatcher` handle alive in
  the struct (dropping it stops watching). Config flag `YODA_WATCH_LABELS=1` default **on**;
  off switch for network filesystems where inotify is unreliable/noisy (NFS often doesn't
  deliver events — document that limitation prominently, it's the same deployment where
  external edits are most common; the rebuild button is the universal fallback).

## Testing

- Stage 1 Axum test: modify a label file on disk directly → GET index (stale) → POST rebuild →
  poll status to ready → GET index reflects change.
- Stage 2: integration test gated `#[cfg(target_os = "linux")]` in CI — touch a label file,
  await ≤2 s, assert index updated. (Watcher tests are inherently timing-flaky; generous
  timeouts, retry once.)

## Risks

- inotify watch limits on huge trees (`fs.inotify.max_user_watches`) — watching only
  `label_root` (not images) keeps counts manageable; log a warning if watcher setup fails and
  continue without it (never fatal).
