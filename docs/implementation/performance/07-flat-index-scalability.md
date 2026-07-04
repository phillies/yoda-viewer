# Flat Index Scalability: Measure First, Then Guard

> Source: optimizations-and-features.md §3.7 · Effort: S (instrumentation) — deliberately NOT
> a rewrite · Depends on: nothing

## Problem statement (and why we're not fixing it yet)

`/api/tree/flat` ships **every** node to the client at startup
(`crates/yoda-web/src/lib.rs:405-412`), and the client's `compute_visible_rows` /
`compute_filtered_rows` (`crates/yoda-ui/src/lib.rs:436-572`) do full scans over all nodes per
interaction. This has a real ceiling — but the ceiling is unknown, the architecture below it
is pleasantly simple, and premature server-side paging would complicate filtering, search
(features/06), and prev/next (features/01), all of which *benefit* from a full client-side
index. So: instrument, warn, and define the escalation trigger.

## Work items

1. **Server instrumentation.** `scan_dataset_tree` timing + counts are partially logged
   already (`dataset tree scan complete`, lib.rs:101). Add: serialized size of the flat-index
   response (log once at startup: `serde_json::to_vec(&resp).len()`), and class-index
   build time in `ClassIndex::load_or_build`.
2. **Client-side timing (dev only).** `web_sys::Performance` marks around the initial
   `fetch_flat_index` + first `compute_visible_rows`; log to console when
   `cfg(debug_assertions)`.
3. **Soft warning in the UI.** If `image_count > 50_000` (constant, adjust with data), show a
   dismissible banner: "Large dataset (N images) — tree filtering may be slow; see
   docs/implementation/performance/07." Uses the existing `.message info` styling.
4. **Micro-optimizations that don't change architecture** (take only if measurements say so):
   - `compute_filtered_rows` runs the matching-image scan on every memo invalidation,
     including expand/collapse. Split the memo: `matching_image_ids` depends only on
     `(class_index, filter_classes, filter_mode)`; row collection depends on that + expansion.
     Two `use_memo`s make expand/collapse O(visible) instead of O(dataset).
   - `VisibleRow` clones `name`/`path` Strings per row; visible rows are bounded by expansion,
     so fine — but switch to `Arc<str>` in `FlatNode` if profiling shows clone pressure.
5. **Documented escalation path** (implement only past the ceiling):
   server-side filter evaluation (`GET /api/tree/filtered?classes=…&mode=…` returning matching
   paths + ancestor folders), keeping the full flat index server-only; client virtualizes the
   returned rows. Prerequisite: virtual scrolling for the tree (render only on-screen rows) —
   which is worth doing *before* any protocol change if raw DOM row count is the actual
   bottleneck (likely: 50k `<div>`s hurt before JSON does).

## Acceptance

- Startup log line: node count, image count, flat-index bytes, scan ms, class-index ms.
- Banner appears on synthetic 60k-image dataset (script to generate: nested temp dirs of empty
  `.jpg` files — add `scripts/gen-synth-dataset.ps1`/`.sh` as part of this task; also useful
  for performance/01 measurements).
- No behavior change below the threshold.

## Risks

None — instrumentation only. Main risk is *skipping* this and discovering the ceiling in a
user report instead of a log line.
