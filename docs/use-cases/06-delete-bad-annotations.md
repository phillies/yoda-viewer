# Use Case 06 — Delete Incorrect Annotations

> **Goal:** remove spurious annotations (duplicates, hallucinated objects, degenerate
> shapes) so they stop polluting training data.
> **Actor:** annotator, reviewer.
> **Mode:** **unlocked** (editing).

## Preconditions

Writable label directory; the bad annotation identified (use cases
[02](02-review-annotation-quality.md) / [04](04-focus-in-crowded-scenes.md)).

## Walkthrough

1. **Unlock editing** (toolbar → **Unlock Editing**).
2. **Locate the object.** Verify visually before deleting: click the object row (or
   double-click the shape on canvas) to select it and confirm the white dashed highlight
   marks the shape you mean. For overlapping duplicates, toggle each candidate's *Visible*
   button to see which annotation is which.
3. **Delete** — either:
   - the **Delete** button on the object's row (works with or without selection), or
   - select the object and press **`Delete`** / **`Backspace`**.
4. **Verify.** The object disappears from canvas and list; the status shows *Labels saved*;
   the object count drops; remaining objects renumber sequentially (`#1…#n` — numbering is
   positional, not stable identity).
5. **Repeat / lock.** Continue on other objects/images; lock editing when done.

## Behavior details

- Deletion rewrites the label file immediately, preserving the order of remaining lines.
- If the deleted object was selected, selection clears; any stale selection is repaired
  automatically.
- Deleting the last annotation leaves an **empty label file** (not a deleted file) — this is
  the correct YOLO convention for "image with no objects".
- Locked mode blocks deletion at both UI and state level
  (*"Unlock editing to delete objects."*).

## Current limitations

- **No undo** — a deleted annotation is gone; recover only from your own backups/VCS
  ([designed](../implementation/features/19-undo-redo.md)).
- No confirmation dialog; the delete button acts immediately.
- One at a time — no multi-select batch delete
  ([designed](../implementation/features/13-multi-select-batch-ops.md)).
- Renumbering means "object #4" in your notes may refer to a different object after a
  deletion in the same image — delete highest-numbered findings first if working from a list.
- Same malformed-file caution as class changes: an image that loaded as 0 objects due to a
  parse error will have its remaining valid lines erased by any save
  ([fix designed](../implementation/correctness/01-partial-label-parse.md)).

## Related use cases

- [05 — Correct object classes](05-correct-object-classes.md)
- [07 — Add missing annotations](07-add-missing-annotations.md)
