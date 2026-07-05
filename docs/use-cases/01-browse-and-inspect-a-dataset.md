# Use Case 01 — Browse and Inspect a Dataset

> **Goal:** get a first look at a YOLO dataset — what's in it, how it's organized, and what
> the annotations look like on the actual images.
> **Actor:** ML engineer / data scientist receiving or revisiting a dataset.
> **Mode:** read-only (default locked mode is sufficient).

## Preconditions

- YoDa is running (web, desktop, or Docker — see use cases [09](09-serve-a-remote-dataset.md)
  / [10](10-desktop-local-review.md)).
- `YODA_IMAGE_BASE_PATH` and `YODA_LABEL_BASE_PATH` point at the dataset; label files mirror
  the image folder structure.
- Optional but recommended: `YODA_CLASS_INFO_YAML` so classes show names instead of
  `class <id>`.

## Walkthrough

1. **Open the app.** The left panel shows the dataset folder tree (folders first,
   alphabetical, hidden files excluded). The startup log prints the total node and image
   counts.
2. **Explore the structure.** Click folders to expand/collapse. Typical YOLO datasets show
   their split at the top level (`train/`, `val/`, `test/`) — this is your first sanity
   check that the dataset is laid out as expected.
3. **Open an image.** Click any image row. The viewer loads the image with segmentation
   overlays on (default), and the status bar shows the file name, pixel dimensions, and
   object count.
4. **Read the annotations.**
   - The *Classes* panel (right) shows each class with its color — hover the image and match
     colors to regions.
   - Toggle **Class Name** in the toolbar to print each object's class directly on the image;
     toggle **Class ID** if you work with numeric ids.
   - Toggle **BBox** to see bounding boxes — for polygon labels this draws their derived
     bounding box (dashed), which quickly shows whether masks and box-style training data
     would agree.
5. **Look closer.** Mouse-wheel to zoom (cursor-centered, up to 6×), drag to pan,
   double-click to reset the view.
6. **Step through images.** Click through the tree image by image. The object list (right)
   gives you the per-image annotation inventory at a glance: count, classes, and whether
   each is a polygon or a bbox.

## Expected results

- Every image in supported formats (`.jpg .jpeg .png .bmp .webp`) is reachable in the tree.
- Images with a missing or empty label file display cleanly with *Objects: 0* — that is
  normal, not an error.
- Unknown class IDs (present in labels but not in the YAML) display as `class <id>` with an
  auto-generated color.

## Current limitations

- No prev/next buttons or arrow-key navigation — stepping through images means clicking tree
  rows ([planned](../implementation/features/01-prev-next-navigation.md)).
- No thumbnails; the tree is names-only
  ([planned](../implementation/features/22-thumbnail-grid.md)).
- A label file with even one malformed line currently shows as *Objects: 0*
  ([fix designed](../implementation/correctness/01-partial-label-parse.md)) — if an image you
  expect to be labeled shows zero objects, inspect its `.txt` by hand.
- Zoom/pan and display toggles reset when you switch images or reload.

## Related use cases

- [02 — Review annotation quality](02-review-annotation-quality.md)
- [03 — Find images by class](03-find-images-by-class.md)
- [08 — Configure dataset metadata](08-configure-dataset-metadata.md)
