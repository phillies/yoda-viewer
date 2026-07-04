# Previous / Next Image Navigation

> Source: next-features.md §2.1 · Effort: S
> Depends on: nothing · Pairs with: features/02 (arrow keys), features/03 (tree auto-scroll),
> performance/06 (image 304s make paging instant)

## Design

Navigation order = the image sequence in the currently displayed tree, i.e. respect the
active class filter and filename search. That makes prev/next a *review* tool ("step through
every image containing `wheel`"), not just directory order.

### Where the sequence lives

The client already has everything: `flat_nodes` + the filter → the `visible_rows` /
`compute_filtered_rows` memo (`crates/yoda-ui/src/lib.rs:677-694`). But visible rows depend on
folder expansion — stepping must not require folders to be expanded. Add a second memo:

```rust
/// Ordered image paths for navigation: all images passing the current filter,
/// in flat-index (depth-first) order, ignoring expansion state.
let nav_sequence: Memo<Vec<String>> = use_memo(move || { … });
```

Implementation: iterate `flat_nodes` filtering `kind == Image`, applying the same match logic
as `compute_filtered_rows` (extract the per-node predicate into a shared
`fn image_matches_filter(node, class_index, filter_classes, filter_mode) -> bool` so the two
can't drift), plus the filename query from features/06 when present.

### Stepping

```rust
fn step(current: &Option<String>, seq: &[String], delta: isize) -> Option<String>
```

- current not in sequence (e.g. filter changed since load) → `delta > 0` picks first element,
  `delta < 0` picks last.
- Ends: clamp (no wraparound; wrapping surprises during QA counts). Disable the button at the
  ends; keyboard no-ops.
- Selecting: reuse the existing `selected_image_path.set(Some(path))` flow — the
  `use_effect` at lib.rs:644 already handles fetching.

### UI

- Toolbar group: `◀ Prev` / `Next ▶` buttons + a position indicator `“137 / 4 210”`
  (index in `nav_sequence` when current is a member, else just total).
- Keyboard `←` / `→` via features/02.
- While `image_loading` is true, ignore step inputs (prevents request pile-up when holding the
  key; alternatively debounce and cancel stale loads — v1: ignore).

## Testing

- Unit-test `step` (pure): middle, ends, empty seq, current-not-in-seq both directions.
- Unit-test `nav_sequence` construction with a filter active (shares fixtures with the
  `compute_filtered_rows` tests).
- E2E (infra/03 v2): open image, press `→`, assert status-bar image name changed.

## Risks

None notable. Watch that `nav_sequence` doesn't recompute per keystroke of unrelated state —
it's derived from the same inputs as the filtered rows memo (see performance/07 memo-split note).
