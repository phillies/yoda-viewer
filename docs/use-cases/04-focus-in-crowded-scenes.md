# Use Case 04 — Focus on Specific Objects in Crowded Scenes

> **Goal:** make a densely annotated image readable — dozens of overlapping masks/boxes make
> it impossible to judge any single annotation; strip the view down to what you're checking.
> **Actor:** annotator / reviewer working on cluttered imagery (street scenes, shelves,
> aerials, microscopy).
> **Mode:** read-only.

## Preconditions

An image with many overlapping annotations is open (see
[use case 01](01-browse-and-inspect-a-dataset.md)).

## Walkthrough — layering the tools

Work from coarse to fine:

1. **Cut overlay types.** Toolbar: turn **BBox** off (boxes over masks double the clutter);
   keep **Mask** on. Turn text chips (**Class ID** / **Class Name**) off while judging
   geometry — they cover boundaries.
2. **Cut whole classes.** *Classes* panel → *Hide* on every class you're not reviewing.
   E.g. hide `road` and `sky` mega-masks to see the small objects they cover. One click per
   class re-shows it.
3. **Cut individual objects.** *Objects* panel → *Visible* button per row. Typical move:
   hide the two overlapping candidates one at a time to see which of the duplicate
   annotations is the accurate one.
4. **Highlight instead of hiding.** Clicking an object row selects it — white dashed outline
   + stronger fill on the canvas — which is often enough to track one object among many
   without hiding anything. In unlocked mode, double-clicking a shape on the canvas selects
   it too (topmost shape wins on overlaps).
5. **Zoom in.** Wheel-zoom onto the region; overlays scale with the image, and label text
   stays proportional. Double-click to reset (locked mode).

## Interaction details worth knowing

- Class-level and object-level visibility **combine**: an object renders only if neither its
  class nor the object itself is hidden. The object row's button shows the *effective* state
  (it reads *Hidden* while the class is hidden, even if the object flag is on).
- Hiding affects the display only — object count in the status bar and the filter index are
  unaffected; nothing is written to disk.
- Visibility is per-session: switching images keeps class-level hiding but resets per-object
  flags (they live in the loaded labels); a reload resets everything.

## Current limitations

- Visibility preferences don't survive a reload
  ([designed](../implementation/features/14-visibility-persistence.md)).
- No "solo" mode (show-only-this in one click) — approximate it by hiding all other classes.
- No multi-select to hide several objects at once
  ([designed](../implementation/features/13-multi-select-batch-ops.md)).
- Double-click select requires unlocked mode, and while unlocked the double-click view-reset
  is disabled ([fix designed](../implementation/correctness/07-view-state-sync.md)).

## Related use cases

- [02 — Review annotation quality](02-review-annotation-quality.md)
- [03 — Find images by class](03-find-images-by-class.md)
