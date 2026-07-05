# Use Case 05 — Correct a Mislabeled Object's Class

> **Goal:** fix "right shape, wrong class" annotations — the most common label defect — with
> the change persisted to the YOLO file immediately.
> **Actor:** annotator, reviewer fixing findings from a QA pass.
> **Mode:** **unlocked** (editing).

## Preconditions

- The label directory is writable by the YoDa process (in Docker: mounted read-write).
- A class map YAML is configured — the dropdown lists classes by name; without it you pick
  numeric `class <id>` entries.
- You have identified the wrong object (use cases [02](02-review-annotation-quality.md) /
  [04](04-focus-in-crowded-scenes.md)).

## Walkthrough

1. **Unlock editing.** Toolbar → **Unlock Editing**; the status pill turns green
   (*Unlocked*), the status bar mode reads *Edit*. Lock state persists as you move between
   images, so a fixing session unlocks once.
2. **Identify the object.** Click its row in the *Objects* panel or double-click its shape
   on the canvas — the selection highlight confirms you have the right one. (Selection is
   optional for this operation; it's for your certainty, not the mechanics.)
3. **Reassign the class.** In the object's row, open the class dropdown and pick the correct
   class. The dropdown contains every class from the YAML plus any class IDs already present
   in the image.
4. **Verify the save.** The status message shows *Labels saved*; the object row and overlay
   recolor to the new class immediately. The YOLO `.txt` on disk has been rewritten — only
   the class token changes semantically; coordinates are re-serialized from the stored
   normalized values (6 decimals).
5. **Continue or lock.** Fix the next object/image, or click **Lock Editing** when done to
   return to safe read-only mode.

## Safety model

- While locked, the dropdown is disabled *and* the state layer independently blocks the
  mutation (a message like *"Unlock editing to change classes."* appears if a blocked path
  is triggered) — belt and braces.
- Saves are atomic per action from the app's perspective, but there is **no undo**: the
  previous class is not recorded anywhere. Re-fix by hand if you misclick.
- The class-index cache updates with the save, so class-filter results stay correct.

## Current limitations

- No undo/redo ([designed](../implementation/features/19-undo-redo.md)).
- One object at a time — no multi-select or "reassign all X in this image to Y"
  ([designed](../implementation/features/13-multi-select-batch-ops.md)), and no dataset-wide
  class merge ([designed](../implementation/features/21-class-operations.md)).
- Save feedback is fire-and-forget: a *failed* save leaves the UI showing the new class while
  the file still has the old one, with only an error message to warn you
  ([fix designed](../implementation/features/09-save-status-feedback.md)).
- **Caution:** if an image's label file had malformed lines, YoDa loaded it as empty — an
  edit+save would then write the file *without* the lines it couldn't parse
  ([fix designed](../implementation/correctness/01-partial-label-parse.md)). If an
  annotated-looking image shows 0 objects, don't edit it; inspect the `.txt` first.

## Related use cases

- [06 — Delete bad annotations](06-delete-bad-annotations.md)
- [07 — Add missing annotations](07-add-missing-annotations.md)
