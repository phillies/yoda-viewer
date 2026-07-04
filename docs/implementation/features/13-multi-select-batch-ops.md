# Multi-Select + Batch Reclass / Batch Delete

> Source: optimizations-and-features.md §5.6 + next-features.md §3.2 · Effort: M
> Depends on: features/09 (single save for batch), features/19 (undo makes batch ops safe —
> strongly recommended first or together)

## Design

### Selection model

Replace `selected_object_index: Option<usize>` with a set, keeping single-select ergonomics:

```rust
pub selected_objects: BTreeSet<usize>,     // on AppState
```

- Compatibility shim during migration: `fn selected_object_index(&self) -> Option<usize>`
  returning the single element when len==1 (callers: delete-enabled check, render selected
  styling, `RenderOptions.selected_index` — extend that to `selected_indices: BTreeSet<usize>`
  in `yoda-core::RenderOptions` so all selected shapes highlight).
- Actions: rework `ToggleSelection`/`SelectLabel` into
  - `Select { label_index, mode: SelectMode }` with
    `enum SelectMode { Replace, Toggle /*ctrl*/, Extend /*shift range*/ }`
  - `SelectAllOfClass(u32)`, `ClearSelection`.
- Range-extend (`Extend`): range over **object-list order** (label index order) from the
  anchor (last `Replace`-selected index, stored as `selection_anchor: Option<usize>`).

### Input wiring

- Object list rows (`ObjectRow` onclick): plain click = Replace; ctrl/cmd-click = Toggle;
  shift-click = Extend. `Event<MouseData>` exposes `modifiers()` in Dioxus — pass
  `(index, SelectMode)` through the `onselect` handler.
- Canvas dblclick (HitAreaShape / inline overlay): same modifier mapping.
- Class legend rows get a "select all" affordance (small `⊙` button) →
  `SelectAllOfClass`.
- `Escape` → `ClearSelection` (features/02 escape cascade: cancel draw > clear selection).

### Batch operations (reducer)

- `ChangeClassForSelection(u32)`: iterate `selected_objects`, set class, single
  `save_effect_for_current_image` at the end (one PUT, one index update — the effect already
  snapshots all labels).
- `DeleteSelection`: delete all selected. **Reindexing pitfall**: `delete_label` deletes one
  index and renumbers (`crates/yoda-core/src/label.rs:271-282`), so deleting a set by
  repeated single deletes corrupts targets. Add
  `pub fn delete_labels(labels: &[LabelObject], indices: &BTreeSet<usize>) -> Vec<LabelObject>`
  in `yoda-core` (filter + renumber once). Clear selection after.
- Locked mode: both blocked via the existing pattern (`ActionResult::blocked`).

### Batch UI

When `selected_objects.len() > 1`, show a compact action bar pinned above the object list:
`“5 selected”  [class ▼ Apply]  [Delete]  [✕]`. The class dropdown reuses `class_options`.
Per-row dropdown/delete stay functional for single objects.

## Testing

Reducer tests: toggle/extend semantics incl. anchor behavior; select-all-of-class; batch
reclass emits one effect; `delete_labels` renumbering (delete {0,2} of 4 → remaining old 1,3
become 0,1); locked blocks; selection cleared after batch delete; stale selections pruned on
`ImageLoaded` (existing behavior generalized to the set).

## Risks

- `RenderOptions` signature change ripples into `yoda-web` fallback + snapshot test — small,
  mechanical (set with 0/1 elements reproduces old behavior; snapshot unchanged if sample uses
  single selection).
- Without undo (features/19), batch delete is a foot-gun — if shipping first, gate batch
  delete behind a two-click confirm in the action bar.
