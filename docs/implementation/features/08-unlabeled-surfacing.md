# Surface Unlabeled Images and Orphan Label Files

> Source: optimizations-and-features.md §5.1 · Effort: S–M
> Depends on: class index (exists) · Related: features/20 (stats) consumes the same data

## Problem

"Which images have no labels?" and "which label files have no image?" are everyday dataset-QA
questions the existing data already answers, but the UI doesn't.

## Design

### Unlabeled images (client-side only — no server change needed)

`AppState`/signal `class_index` maps every image path → `Vec<u32>`; an **empty vec** means the
label file was missing or empty (`extract_class_ids_from_label_file` returns `[]` for both,
`crates/yoda-data/src/class_index.rs:117-130`).

1. **Tree badge**: in `FlatNodeView`, when `kind == Image` and
   `class_index.get(path).is_none_or(|v| v.is_empty())`, render a small hollow-circle badge
   (`○`, muted color, `title="no labels"`) after the label. Pass a precomputed
   `is_unlabeled: bool` prop from the rows memo to keep the component dumb.
2. **Filter chip**: an "Unlabeled" pseudo-chip in `ClassFilterBar` (visually separated from
   class chips). State: `filter_unlabeled: bool` — UI-local or `AppState`, consistent with
   wherever `filter_classes` ends up after performance/01. Predicate composes into
   `image_matches_filter` (features/06): `unlabeled ⇒ class list empty`. Combining with class
   chips is contradictory — when the unlabeled chip is active, disable the class chips
   (clearest semantics).
3. **Count in the status area**: "N unlabeled" derived from the index (memo), shown in the
   filter bar header.

Distinguish "missing label file" from "empty label file"? Not in v1 — same practical meaning.
(features/20 can split them server-side later.)

### Orphan label files (server-side)

Labels with no corresponding image are invisible to the flat index (it scans the image root
only). Detect during `ClassIndex::load_or_build`:

- Walk `label_root` for `*.txt` (reuse `walk_dir`, skip `.yoda_class_index.json` — dot-file
  already skipped by hidden filter? `walk_dir` does **not** filter hidden files — add the
  filter or match extension).
- An orphan = label path whose mirrored image path (inverse of `map_image_to_label_path`:
  strip label root, try each `IMAGE_EXTENSIONS`) matches no image in the flat index.
- Store `orphans: Vec<String>` on `ClassIndex` (serialized with the cache) and expose via
  `GET /api/class-index` response (`orphan_labels: Vec<String>`, serde-default for
  compatibility).
- UI v1: a warning line in the filter bar when non-empty — "⚠ 12 label files have no image"
  with `title` listing the first few; full list can wait for features/20's stats page.

## Testing

- Unit (`yoda-data`): fixture with labeled image, unlabeled image (no txt), empty-txt image,
  and an orphan txt → index reflects all four; orphan inverse-mapping tries all extensions.
- UI unit: predicate + chip-disabling logic.
- Manual: badge/chip/count on the example dataset after deleting one label file.

## Risks

- The orphan scan adds a label-root walk to index build — trivial next to per-file reads.
- Inverse mapping is ambiguous if two images (`a.jpg` and `a.png`) share a stem — then the txt
  isn't an orphan at all; the "matches no image" rule handles this correctly by construction.
