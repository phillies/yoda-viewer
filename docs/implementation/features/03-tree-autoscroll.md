# Tree Auto-Scroll + Auto-Expand to the Selected Image

> Source: next-features.md §2.4 · Effort: S
> Depends on: features/01 (main beneficiary — keyboard paging moves selection off-screen)

## Problem

When the selected image changes by any means other than clicking its tree row (prev/next,
last-image restore, future stats-panel deep links), the tree neither expands the ancestor
folders nor scrolls the row into view.

## Design

Two halves: expansion (Rust state) and scrolling (DOM).

### 1. Auto-expand ancestors

On `selected_image_path` change, ensure every ancestor folder id is in `expanded_dirs`:

```rust
fn ancestor_ids(nodes: &[FlatNode], path: &str) -> Vec<u32>   // walk parent_id chain
```

- Find the node by path (build a `path → id` HashMap memo next to `children_map` — also
  useful elsewhere), then walk `parent_id` upward (`FlatNode.parent_id`,
  `crates/yoda-data/src/lib.rs:49-56`).
- Apply in a `use_effect` watching `selected_image_path`; only *insert* ids (never collapse).
- Filtered mode: `compute_filtered_rows` respects `expanded_dirs` the same way — no special
  case. If the selected image doesn't match the active filter it simply won't render; do not
  auto-clear the filter (surprising); the status bar already shows the current image name.

### 2. Scroll into view

The rows are plain divs inside `.tree-scroll`. Give each image row an id:
`id: "tree-node-{row.id}"` in `FlatNodeView` (`yoda-ui/src/lib.rs:1190`). After the expand
effect runs, scroll via `document::eval`:

```rust
document::eval(&format!(
    "document.getElementById('tree-node-{id}')?.scrollIntoView({{block:'nearest'}})"));
```

- `block: 'nearest'` avoids yanking the viewport when the row is already visible.
- Timing: the row must exist in the DOM post-expansion. Dioxus applies signal-driven DOM
  updates before subsequently spawned tasks run their next await point; to be safe, run the
  eval from a spawned task after a `TimeoutFuture`/next-tick yield, or use `onmounted` on the
  row when `is_selected` (Dioxus `onmounted` gives an element handle with
  `scroll_to(ScrollBehavior::Smooth)` — **preferred**, no eval, no timing guess):

```rust
onmounted: move |el| { if is_selected { spawn(async move { let _ = el.scroll_to(…).await; }); } }
```

  Note `onmounted` fires on mount only — row already mounted but off-screen won't refire; so
  combine: `use_effect` on selection change handles the mounted case via eval, `onmounted`
  handles the just-expanded case. (Or accept eval-only with a 1-frame delay — simplest, ship
  that first.)

## Testing

- Unit: `ancestor_ids` on a nested fixture (root → a → b → img) returns `[a_id, b_id]`
  order-independent.
- Manual/E2E: collapse everything, press `→` repeatedly across a folder boundary — selection
  stays visible; tree expands the new folder.

## Risks

- Smooth-scroll spam when holding `→`: use `block:'nearest'` + instant (not smooth) behavior
  to keep up.
