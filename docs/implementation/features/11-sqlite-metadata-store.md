# SQLite Metadata Store (replaces the JSON cache; enables workflow features)

> Source: optimizations-and-features.md §6.4 · Effort: L
> Depends on: nothing hard · Supersedes: performance/04 stage 2
> Enables: features/19 (persistent history), features/20 (stats cache), features/22
> (thumbnail bookkeeping), bold/04 (review states), bold/05 (embeddings)

## Principle

YOLO `.txt` files remain the **single source of truth for geometry**. The database is a
rebuildable cache + workflow layer. Deleting it must never lose annotations — only derived
data and workflow metadata (review states are workflow, not geometry; accept that they live
only in the DB and say so).

## Choice: `rusqlite` (bundled) over `redb`

- SQL queries are exactly what features/20 (stats) needs; `redb` would push aggregation into
  Rust code.
- `rusqlite` with `bundled` feature avoids system-lib headaches on all three OSes and in the
  Docker image.
- Concurrency: WAL mode; single-writer semantics fit the app (one process). Wrap in a small
  `Db(Mutex<Connection>)` — or `r2d2_sqlite` pool if the stats page shows contention (start
  with Mutex).

## Location

`<label_root>/.yoda/yoda.db` (new dot-dir; also the future home of thumbnails and backups —
migrating the class-index JSON path). Read-only label mounts: fall back to a
platform cache dir (`directories` crate) keyed by dataset id, or disable (features/12
read-only mode disables writes anyway). `YODA_DB_PATH` override env var.

## Schema v1

```sql
CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);          -- schema_version, dataset roots
CREATE TABLE image (
  path TEXT PRIMARY KEY,          -- dataset-relative, forward slashes
  mtime INTEGER, size INTEGER,    -- label-file validators for incremental refresh
  class_ids BLOB,                 -- packed u32 LE array (or JSON; BLOB is smaller)
  label_count INTEGER,
  unlabeled INTEGER GENERATED ALWAYS AS (label_count = 0)
);
CREATE INDEX idx_image_unlabeled ON image(unlabeled);
```

Class-ids-per-image replaces `ClassIndex.entries`; `mtime/size` make refresh incremental:
rescan only files whose validators changed — this retroactively speeds up features/10's
rebuild from O(all reads) to O(changed reads).

## Migration plan

1. New crate `yoda-db` (keeps `rusqlite` out of `yoda-data`'s dependents until stable):
   `Db::open`, `Db::upsert_image`, `Db::class_index_snapshot() -> ClassIndex`,
   `Db::matching_images(filter)`.
2. `ClassIndex` becomes a *view* produced from the DB; `load_or_build` becomes
   `Db::sync(flat_index, label_root, progress_cb)` using validators. Keep the public
   `ClassIndex` type so `yoda-app`/`yoda-ui` don't change.
3. One-time import: if `.yoda_class_index.json` exists and DB is empty, import entries
   (without validators → first sync re-reads files; acceptable), then rename the JSON to
   `.yoda_class_index.json.migrated`.
4. Save path (`yoda-web` `save_labels`): replace index-insert + JSON-save with one
   `upsert_image` (point write — performance/04's problem dissolves; remove the flusher).
5. Schema versioning: `meta.schema_version`; on mismatch, drop derived tables and resync
   (it's a cache — no migration ceremony until workflow tables arrive in bold/04, which DO
   need real migrations; note that boundary in `yoda-db`'s README section).

## What NOT to put in the DB (v1)

Label geometry, class names (YAML stays authoritative), color maps, view state. Resist the
gravity — every table added before its feature lands is speculative.

## Testing

- `yoda-db` unit tests with tempdir DBs: sync from fixture dataset; validator-based
  incrementality (touch one file → only it re-read; assert via read counter injected in the
  callback); JSON import path; corrupt DB file → recreated cleanly (open failure ⇒ delete +
  rebuild, log warning).
- Existing Axum tests keep passing with the swapped backend.

## Risks

- Bundled SQLite compile time (+~30 s cold). Fine.
- Two processes on one dataset (two yoda instances) — WAL tolerates it; last-writer-wins is
  the same semantics as today's JSON. Document as unsupported-but-not-corrupting.
