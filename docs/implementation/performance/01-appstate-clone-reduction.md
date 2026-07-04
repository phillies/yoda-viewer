# Stop Cloning `AppState` (and the Whole-Dataset Class Index) Every Render

> Source: optimizations-and-features.md §3.1 · Effort: M · Priority: highest perf item
> Depends on: nothing · Enables: smooth UX on 100k-image datasets

## Problem

`App()` (`crates/yoda-ui/src/lib.rs:696`) does `let state_value = app_state();` — a full
`AppState` clone on **every** render. `AppState` contains
`class_index: HashMap<String, Vec<u32>>` with one entry per dataset image
(`crates/yoda-app/src/lib.rs:77`), plus `current_labels`, `class_map`, etc. Every toolbar
toggle, tree expansion, or cursor-driven re-render clones the entire map. On 100k images
that's tens of MB of allocation per interaction. Secondary offenders:

- `visible_labels()` clones all labels per render (`yoda-app/src/lib.rs:120-129`).
- `render_overlay_data_uri(...)` rebuilds + percent-encodes the SVG per render
  (`yoda-ui/src/lib.rs:698`), even for renders that didn't touch labels/options.
- `color_map()` and `class_options.clone()` per row in the object list.

## Design

Three independent steps, in order of payoff:

### Step 1 — Evict `class_index` from `AppState`

The reducer never *mutates* it except `ClassIndexLoaded`; its only consumer is the
`compute_filtered_rows` memo (lib.rs:677-694). Move it to a UI-level signal:

- Delete `AppState.class_index` and `AppAction::ClassIndexLoaded`.
- `App` holds `let class_index = use_signal(HashMap::new);` set once by the startup fetch.
- The `visible_rows` memo reads `class_index.read()` directly.
- `filter_classes`/`filter_mode` stay in `AppState` (they're small and reducer-owned).

This alone removes the O(dataset) clone.

### Step 2 — Read, don't clone

Replace `let state_value = app_state();` with scoped reads:

- Rsx blocks that need several fields: `let state = app_state.read();` and borrow
  (`state.show_bbox`, etc.). Dioxus signals track reads for reactivity either way; the clone
  buys nothing.
- Where the borrow fights the borrow-checker across event-handler closures, clone **fields**,
  not the struct (e.g. `let class_map = app_state.read().class_map.clone();` — small).
- Per-row props: `class_options` is cloned per `ObjectRow` (lib.rs:1042); wrap it in
  `Rc<Vec<(u32, String)>>` (or `use_memo`) and clone the `Rc`.

### Step 3 — Memoize derived values

```rust
let visible_labels = use_memo(move || app_state.read().visible_labels());
let overlay_uri = use_memo(move || render_overlay_data_uri(
    &app_state.read(), &color_map.read(), &visible_labels.read()));
```

`use_memo` re-runs only when its tracked signals change; renders triggered by
`expanded_dirs`, `tree_loading`, `selected_image_path`, or draw-cursor movement no longer
touch the SVG path. Note the draw overlay's `onmousemove` sets a `cursor` signal inside
`CanvasOverlay` — verify that signal is component-local (it is: `use_signal` at lib.rs:1306)
so cursor moves don't re-render `App`. Keep it that way.

## Measurement

Before/after: `console.time` around a toolbar toggle with the example dataset ×1 and with a
synthetic 100k-entry class index (add a dev-only generator or use a large dataset).
Target: toggle re-render < 5 ms independent of dataset size.

## Testing

- Behavior parity: existing reducer tests unaffected. Filter behavior covered by manually
  exercising chips after Step 1 (plus keep `compute_filtered_rows` unit-testable — it's
  already a pure function; add a test if not covered).
- Watch for stale-memo bugs: after `PUT /api/labels` succeeds, `run_effects` mutates
  `status` — confirm overlay memo doesn't depend on `status` (it reads specific fields only
  after Step 3's refactor: pass fields, not `&AppState`, into `render_overlay_data_uri` to
  make dependencies explicit).

## Risks

- Dioxus 0.7 memo semantics: `use_memo` requires `PartialEq` on the output; `Option<String>`
  (data URI) qualifies. If the overlay memo output is large, equality checks are string
  compares — still far cheaper than regeneration.
- Borrowing across `rsx!` boundaries can be fiddly; prefer small field clones over fighting
  lifetimes.
