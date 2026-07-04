# Show Label-Only Classes in the Filter Bar (use `all_class_ids`)

> Source: next-features.md §1.6 · Effort: S
> Depends on: nothing

## Problem

`/api/class-index` returns `all_class_ids` — the union of every class ID found in label files
(`ClassIndex::all_class_ids`, `crates/yoda-data/src/class_index.rs:92-94`) — but the client
fetches and discards it (`#[allow(dead_code)]` on `ClassIndexResponse.all_class_ids`,
`crates/yoda-ui/src/lib.rs:352-356`). `ClassFilterBar` derives its chips from
`class_map` only (lib.rs:1092), i.e. classes declared in the dataset YAML. Consequences:

- Classes present in label files but missing from the YAML (typo'd YAML, stale YAML, no YAML
  at all) are **invisible to the filter** — you cannot find the images that contain them,
  which is exactly the QA situation where you'd want to.
- With no `YODA_CLASS_INFO_YAML` configured, `class_map` is empty and the filter bar renders
  nothing at all (`if class_map.is_empty() { return … }`).

## Design

1. Plumb the value: `fetch_class_index` returns the full `ClassIndexResponse` (entries +
   all_class_ids). Store IDs alongside the entries — either extend
   `AppAction::ClassIndexLoaded(HashMap<…>)` to carry `(entries, Vec<u32>)`, adding
   `pub dataset_class_ids: BTreeSet<u32>` to `AppState`, or (simpler given performance/01 will
   move `class_index` out of `AppState`) keep both in a UI-local signal.
2. Chip list = `class_map keys ∪ dataset_class_ids`, sorted. Chips without a YAML name render
   as `class {id}` (pattern already used everywhere, e.g. lib.rs:1130) and get a subtle
   "undeclared" marker — e.g. a dashed chip border + `title="present in labels but not in the
   dataset YAML"`. That marker doubles as a data-quality signal.
3. Remove the `#[allow(dead_code)]`.
4. Drop the `class_map.is_empty()` early-return; replace with
   `class_map.is_empty() && dataset_class_ids.is_empty()`.

## Server note

`all_class_ids` is computed by iterating all entries on every request (lib.rs `class_index_handler`).
Fine at current scale; if performance/04 introduces a cached/incremental index, maintain the
union incrementally there.

## Testing

- UI-logic unit test for the merged, sorted chip list (extract a pure
  `fn filter_chip_ids(class_map, dataset_ids) -> Vec<u32>`).
- Axum test already covers the endpoint; add an assertion that a label file with class `7`
  absent from the YAML appears in `all_class_ids`.
- Manual: dataset with no YAML → filter bar still shows chips for all found classes.

## Risks

None. Slight UI noise on datasets with garbage label files — the dashed-border marker makes
that a feature (it surfaces the garbage).
