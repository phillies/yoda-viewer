# Polygon Vertex Editing + BBox Resize Handles + Simplification

> Source: optimizations-and-features.md §6.5 + next-features.md §3.7 · Effort: L
> Depends on: performance/02 (inline SVG — hard prerequisite: handles must be DOM),
> features/19 (undo — drag mistakes need cheap recovery), correctness/08 (screen-space
> handle sizing shares the scale math)

## Scope

Three edit affordances on the **selected** object in unlocked+Edit mode:

1. Drag a vertex (polygon) / corner-or-edge handle (bbox).
2. Insert a vertex: click a polygon edge midpoint handle.
3. Delete a vertex: alt-click (or select-vertex + Delete), min 3 vertices enforced.

Plus a one-shot **simplify** action (RDP) for imported model-generated polygons.

## State model (`yoda-app`)

Geometry edits are drag-interactive; running every mousemove through the reducer + a disk
save per pixel is wrong. Split *transient drag* from *committed edit*:

```rust
pub struct VertexDrag { pub label_index: usize, pub vertex: usize, pub pos: Point }
pub vertex_drag: Option<VertexDrag>,       // AppState — transient, never persisted
```

Actions:
- `BeginVertexDrag { label_index, vertex }` (blocked when locked)
- `UpdateVertexDrag(Point)` — updates only `vertex_drag.pos` (cheap, no label mutation)
- `CommitVertexDrag` — writes the point into
  `current_labels[i].pixel_points[v]`, **recomputes** `normalized_coords` (point/w, point/h)
  and `pixel_bbox` (`bounding_box_for_points`), emits the persist effect. One save per
  gesture.
- `InsertVertex { label_index, edge, pos }` / `DeleteVertex { label_index, vertex }` —
  immediate commit + save; delete rejects when `len == 3`.
- `SimplifySelected { epsilon_px: f32 }` — RDP over `pixel_points` (closed-polygon variant:
  run on the ring, keep ≥3 points), recompute normalized, save.

BBox: 8 handles (4 corners, 4 edges). Corner drag moves that corner; edge drag moves one
coordinate. Commit recomputes `[cx, cy, w, h]` normalized (shared code with features/05's
`create_bbox_from_pixels` — reuse its normalizer). Enforce min size, clamp to image.

Invariant note: `LabelObject` now has *derived-field recompute* in several places — add
`impl LabelObject { fn recompute_from_pixels(&mut self, w: u32, h: u32) }` in `yoda-core` as
the single point of truth (also used by performance/03's hydrate).

## Rendering (`yoda-ui`, extends performance/02's `LabelShape`)

For the selected label only:

- Vertex handles: `<circle r={handle_r}>` per point; `handle_r` in screen-constant size via
  correctness/08's scale math.
- Midpoint insert handles: smaller, lower-opacity circles at edge midpoints; visible on
  polygon hover.
- Drag preview: while `vertex_drag` is Some, render the polygon with the dragged point
  substituted (pure render-time substitution — labels themselves untouched until commit).
- Mouse plumbing: `onmousedown` on handle → Begin; overlay-level `onmousemove`/`onmouseup`
  (the existing capture-rect pattern) → Update/Commit. Suppress pan-drag while a handle drag
  is live: reuse the `data-draw-mode` skip or add `data-vertex-drag`.

## Simplify UI

Button in the object action area when a polygon is selected (`Simplify`), epsilon derived
from image size (e.g. `0.002 × max(w,h)`, tunable later). Show point-count delta in the
status text (`214 → 38 points`). Undo covers regret.

## Testing

- `yoda-core`: `recompute_from_pixels` roundtrips; RDP unit tests (collinear removal, square
  survives, min-3 floor, closed-ring correctness).
- Reducer: full drag lifecycle (begin/update×n/commit → one effect), insert/delete vertex
  index bookkeeping, min-vertex rejection, locked blocks, drag cleared on `ImageLoaded`.
- E2E: drag a vertex, assert label file changed (via `/api/labels` GET diff).

## Risks

- Event ordering between handle mousedown and container pan mousedown — the suppress
  attribute handles it, but test on desktop webview too.
- Handle hit targets at low zoom: screen-constant sizing (correctness/08) is what makes this
  usable; don't ship without it.
