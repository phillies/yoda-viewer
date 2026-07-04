# Undo / Redo for Label Edits

> Source: optimizations-and-features.md §6.1 + next-features.md §3.1 · Effort: M
> Depends on: features/09 (save pipeline — undo triggers saves) · Unblocks safe: features/13
> (batch ops), features/16 (vertex editing), features/21 (class ops use a different mechanism)

## Scope

Per-image, in-session history of **label mutations**: class change, delete (single/batch),
new polygon/bbox, vertex edits, simplify. Not undoable: visibility toggles, selection, view,
filters (cheap to redo manually; polluting the stack hurts). History clears on image change
(v1; cross-image later via features/11 if wanted).

## Design: snapshot ring, not command pattern

Label vectors are small (≤ thousands of objects); a command pattern's inverse-operation
bookkeeping isn't worth it. Snapshot the whole `current_labels` around each mutation:

```rust
pub struct History {
    undo: VecDeque<HistoryEntry>,     // capped at 50
    redo: Vec<HistoryEntry>,
}
pub struct HistoryEntry {
    labels: Vec<LabelObject>,
    description: &'static str,        // "delete object", "change class", …
}
```

Field on `AppState` but **excluded from PartialEq/serde** if that causes churn — simplest:
wrap in a struct that implements `PartialEq` as always-equal, or keep history in a separate
`Signal<History>` owned by the UI and fed by the reducer's return value (see below).

### Where snapshots happen — reducer-integrated

Inside `apply_action`, before any mutating branch runs, push the pre-state:

- Identify mutating actions centrally:
  `fn is_label_mutation(action: &AppAction) -> Option<&'static str>` returning the
  description. Push `(current_labels.clone(), desc)` to `undo`, clear `redo` — but **only if
  the action actually succeeds** (blocked/locked actions must not pollute history): push
  tentatively, pop on `blocked`/error, or restructure mutating branches to call a
  `with_history(state, desc, |state| …)` helper. The helper approach reads best.
- New actions `Undo` / `Redo`: swap `current_labels` with the popped entry (pushing the
  current state to the opposite stack), then emit the persist effect
  (`save_effect_for_current_image`) — undo writes through to disk, keeping the
  file-as-source-of-truth invariant. Selection: prune to valid indices (existing logic in
  `delete_label_by_index` generalizes).
- `ImageLoaded`/`ImageCleared` reset both stacks.

### Memory

50 entries × (say) 500 labels × ~50 normalized floats ≈ few MB worst case — fine. Cap
constant `HISTORY_LIMIT: usize = 50`, drop from the front.

## UI

- Toolbar buttons `↶ Undo` / `↷ Redo`, disabled when the respective stack is empty; `title`
  shows the entry description ("Undo: delete object").
- Keys: `Ctrl+Z` / `Ctrl+Shift+Z` (+ `Ctrl+Y`) via features/02 (its serializer already
  planned modifier support).
- Status text on undo/redo: `Undid: change class`.

## Testing

Reducer tests (the pattern of existing tests extends naturally):
- mutate → undo → labels equal pre-state, persist effect emitted;
- undo → redo roundtrip;
- blocked action (locked) pushes nothing;
- new mutation clears redo;
- cap eviction (51 mutations → oldest gone);
- image switch clears history;
- selection pruned after undo of a create.

E2E: change class → Ctrl+Z → dropdown shows original; file on disk reverted (via API read).

## Risks

- Double-persist interplay with features/09's generation counter — undo emits a normal save;
  no special casing needed, verify test coverage overlaps.
- If features/13 lands first, batch ops get history for free via `with_history` — coordinate
  the helper's introduction with whichever PR goes first.
