# Per-Image Class Summary Tooltips in the Tree

> Source: optimizations-and-features.md §5.3 · Effort: XS–S
> Depends on: class index (exists), class map (exists)

## Design

Hovering an image row shows its class composition, e.g. `3× wheel, 1× bumper` — skim the
dataset without opening images.

### Data gap: counts

`class_index` stores **unique** class ids per image, not counts
(`extract_class_ids_from_label_file` dedupes via `BTreeSet`,
`crates/yoda-data/src/class_index.rs:117-130`). Two tiers:

- **v1 (ship first)**: names only from existing data — `title="wheel, bumper"`. Zero backend
  change.
- **v2 (with counts)**: change `ClassIndex.entries` value type to `Vec<(u32, u32)>`
  (class_id, count) — or a parallel `counts` map to avoid breaking the filter code paths.
  Cache-format change → bump/invalidate the JSON cache (or land with features/11 where the
  schema change is free). Count extraction: tally per line instead of set-insert.

### Rendering

Native `title` attribute on the tree-row button in `FlatNodeView`
(`crates/yoda-ui/src/lib.rs:1190-1208`):

- Compute lazily-enough: building the string for every visible row is fine (rows are bounded
  by expansion); pass `tooltip: Option<String>` as a prop from the rows memo where
  `class_index` and `class_map` are both in scope.
- Format: `class names sorted by count desc (v2) / id (v1)`, join ", ", prefix counts in v2;
  unlabeled images (features/08) → `"no labels"`.
- Folder rows: skip in v1. (Aggregated folder summaries mean transitive tallies — nice, but
  belongs to features/20's stats work.)

Native tooltips have ~1 s hover delay and no styling; that's acceptable for v1 and costs one
attribute. If richer tooltips are wanted later (color swatches per class), build a positioned
`div` on `onmouseenter` — defer until asked.

## Testing

- Unit: tooltip formatter (ids → names with fallback `class {id}`, unlabeled case).
- Manual: hover rows on example dataset.

## Risks

None. Verify the `title` attribute passes through Dioxus rsx on a `button` (standard
attribute — it does).
