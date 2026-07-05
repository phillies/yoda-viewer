# Use Case 07 — Add a Missing Annotation (Draw a Polygon)

> **Goal:** annotate an object the dataset missed, as a segmentation polygon saved in YOLO
> format.
> **Actor:** annotator.
> **Mode:** **unlocked** + **Draw** interaction mode.

## Preconditions

Writable label directory; class map YAML configured (so you can pick the class by name).

## Walkthrough

1. **Unlock editing**, then click **Draw Polygon** in the toolbar. The button highlights, a
   vertex counter (`0 pts`) appears, and the cursor becomes a crosshair over the image.
   Pan-dragging is disabled while drawing (clicks place vertices instead) — set your zoom
   *before* entering draw mode, or cancel, adjust, and re-enter.
2. **Pick the class** for the new object in the dropdown next to the draw button (defaults
   to the lowest class ID). You can change it up to the moment you finish the polygon.
3. **Place vertices** by clicking along the object's outline. Live feedback:
   - committed edges render as a dashed polygon with a translucent fill;
   - a dashed "rubber band" line follows the cursor from the last vertex;
   - each vertex shows as a small circle; the **first vertex** grows a ring once you have 3+
     points — that ring is the close zone.
4. **Close the polygon** — any of:
   - click inside the first vertex's ring (it brightens and previews the closing edge when
     you're near);
   - press **Enter**;
   - click the **Finish** button (enabled from 3 vertices).
5. **Or abort** with **Escape** / the **Cancel** button — discards all placed vertices.
6. **Verify.** The new object appears in the overlay and at the end of the object list with
   your chosen class; status shows *Labels saved*. On disk, a new line with normalized
   polygon coordinates was appended to the image's label file (the file and its parent
   directories are created if this was the image's first annotation).

## Behavior details

- Minimum 3 vertices; Finish stays disabled below that, and Enter is ignored.
- Vertices are stored exactly where clicked (image-pixel positions, normalized on save) —
  there is no snapping or smoothing.
- After finishing, the app returns to Edit interaction mode automatically.
- Switching images or locking mid-draw discards in-progress vertices.

## Current limitations

- **No vertex editing after the fact** — misplaced vertices mean delete + redraw
  ([designed](../implementation/features/16-vertex-editing.md)); there is also no undo for
  the finished polygon ([designed](../implementation/features/19-undo-redo.md)).
- **No bbox drawing** — only polygons can be created; box-only datasets can't gain new boxes
  ([designed](../implementation/features/05-bbox-draw-mode.md)).
- The close-zone ring has a fixed size in image pixels — on very large images at low zoom it
  is small and fiddly; zoom in, or use Enter to finish
  ([fix designed](../implementation/correctness/08-close-zone-zoom-scaling.md)).
- No mid-draw zoom/pan (see step 1).
- No model-assisted drawing (click-to-segment)
  ([designed](../implementation/bold/02-sam-click-to-segment.md)).

## Related use cases

- [05 — Correct object classes](05-correct-object-classes.md)
- [06 — Delete bad annotations](06-delete-bad-annotations.md)
