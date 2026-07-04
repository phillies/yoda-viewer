# Filename Search / Filter in the Tree

> Source: next-features.md §2.7 · Effort: S
> Depends on: composes with the class filter and features/01 nav sequence

## Design

A text input in the tree panel (below `ClassFilterBar`) that live-filters visible tree rows by
case-insensitive substring match on the image filename, composing with the class filter
(logical AND).

### State

UI-local signal, not `AppState` — it's pure view state:
`let name_query = use_signal(String::new);` in `App`. (If it lived in `AppState` every
keystroke would run the reducer; unnecessary.)

### Filtering

Extend the row computation (`crates/yoda-ui/src/lib.rs:677-694`):

- Extract today's per-image predicate (class match) into
  `image_matches_filter(...)` (shared with features/01).
- Add `name_matches(node, query)`: empty query → true; else
  `node.name.to_lowercase().contains(&query.to_lowercase())` (lowercase the query once per
  recompute, not per node).
- When *either* filter is active, use the `compute_filtered_rows` path (it already handles
  ancestor-folder inclusion and hiding empty folders) with the combined predicate — i.e.
  generalize its `matching_images` construction to take a closure. When both are empty, keep
  the cheap `compute_visible_rows` path.
- Auto-expansion while searching: with a non-empty query, pass an "expand all matching
  folders" set instead of `expanded_dirs` (compute: all ancestor ids of matches — already
  built inside `compute_filtered_rows` as `matching_folder_ids`). Restore user expansion
  state when the query clears (trivially: never mutated it).

### Debounce

Substring scan over 100k nodes per keystroke is ~ms-scale; still, debounce input → memo by
150 ms to keep typing smooth on big datasets: store raw input in one signal, copy into
`name_query` from a spawned task that sleeps and checks the value is unchanged
(`gloo-timers` `TimeoutFuture` on wasm; or skip debounce v1 and revisit with
performance/07 measurements).

### UI

```
input.filter-input { placeholder: "Filter by filename…", value: "{raw_query}", oninput: … }
+ a ✕ clear button when non-empty
```

Style consistent with `.filter-bar`. Add match count (`“23 matches”`) under the input —
derived from the memo, free.

Keyboard: `/` focuses the input (register in features/02; note the input-focused guard in the
key relay must not swallow `/` before focusing).

## Testing

- Unit: generalized filtered-rows fn with (a) name-only query, (b) class+name combined,
  (c) query matching nothing → empty rows, (d) folder ancestors of matches included &
  expanded.
- E2E: type a filename fragment, assert row count.

## Risks

None significant. Watch interaction with features/01: the nav sequence must use the same
combined predicate (documented there).
