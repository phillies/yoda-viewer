# Bounding-Box Draw Mode (drag a rectangle)

> Source: next-features.md §2.6 · Effort: M
> Depends on: none hard; benefits from performance/02 (single overlay component)

## Problem

The draw toolbar only creates polygons. `LabelType::Bbox` parsing/rendering/persistence all
exist; only the creation gesture is missing.

## Design

### Domain (`yoda-core`)

New constructor beside `create_label_from_pixels` (`crates/yoda-core/src/label.rs:232`):

```rust
pub fn create_bbox_from_pixels(
    p1: (f32, f32), p2: (f32, f32),          // any two opposite corners
    image_width: u32, image_height: u32,
    class_id: u32, index: usize,
) -> Result<LabelObject, LabelError>
```

- Normalize corners (min/max), clamp to image bounds, compute
  `normalized_coords = [cx, cy, w, h]` (YOLO bbox format — matching `parse_bbox`'s inverse),
  `pixel_points = [top-left, bottom-right]`, `pixel_bbox` accordingly.
- Reject degenerate boxes: `w.min(h) < MIN_BBOX_PX (e.g. 2.0)` → new
  `LabelError::DegenerateBbox`.

### State (`yoda-app`)

- `InteractionMode` gains a third variant: `DrawBbox` (rename nothing; `Draw` stays the
  polygon mode — or rename to `DrawPolygon` in the same PR for clarity; renaming touches
  serialization of `AppState` only in-memory, safe).
- New actions:
  - `BeginBboxDrag(Point)` → sets `bbox_drag_start: Option<Point>` (new field)
  - `UpdateBboxDrag(Point)` → sets `bbox_drag_current: Option<Point>` (for preview)
  - `CommitBboxDrag(Point)` → calls `create_bbox_from_pixels(start, end, …,
    drawing_class_id, current_labels.len())`, pushes, clears drag state, returns the
    persist effect (mirror `finish_drawing`, `crates/yoda-app/src/lib.rs:463-494`, including
    the locked-mode block).
  - `CancelDrawing` also clears drag fields.
- Guard rails mirror polygon draw: locked → `ActionResult::blocked(EditOperation::FinishDrawing)`
  (add `EditOperation::CommitBbox` for precision).

### UI (`yoda-ui`)

- Toolbar: second draw button "Draw BBox" next to "Draw Polygon"
  (lib.rs:846-858), active-state styling shared; both hidden when locked (existing block).
- `CanvasOverlay` in `DrawBbox` mode: reuse the full-canvas capture rect (lib.rs:1361-1391)
  with `onmousedown` → `BeginBboxDrag`, `onmousemove` → `UpdateBboxDrag`,
  `onmouseup` → `CommitBboxDrag`. Render a live preview `rect` (dashed, accent color) from
  drag start→current.
- Pan/zoom conflict: `PAN_ZOOM_SCRIPT.onMouseDown` already skips when
  `data-draw-mode === 'true'` (lib.rs:130-133); set that attribute for `DrawBbox` too
  (the rsx currently checks `== InteractionMode::Draw`; change to `is_drawing()` helper on
  `AppState`).
- Mouse leaves canvas mid-drag: `onmouseleave` cancels the drag (clear, don't commit).

### Keyboard

`D` cycles or dedicated keys — recommend `D` = polygon, `Shift+D`/`R` = bbox; register in
features/02's `action_for_key`.

## Testing

- `yoda-core` unit: corner order invariance (all 4 drag directions produce identical
  normalized coords), clamping at edges, degenerate rejection, roundtrip through
  write/parse (`parse_bbox` inverse property).
- Reducer: drag lifecycle appends bbox + emits persist effect; locked-mode block; cancel
  clears state.
- E2E (stretch): drag on canvas, object count +1 with `[bbox]` badge.

## Risks

- Coordinate space: `element_coordinates()` on the capture rect is already in viewBox (image)
  units — same as polygon draw, no extra transform needed. Verify after correctness/08 lands
  (it touches nearby logic).
