# Trim the Label Wire Format (send normalized coords only)

> Source: optimizations-and-features.md §3.3 · Effort: M
> Depends on: nothing · Related: performance/06 (compression) reduces the same pain with less
> surgery — consider doing that first and re-measuring.

## Problem

`LabelObject` (`crates/yoda-core/src/label.rs:45-54`) serializes `normalized_coords` **and**
`pixel_points` **and** `pixel_bbox`. The pixel data is a pure function of normalized coords +
image dimensions, and `LabelsResponse` already carries `width`/`height`. Effect: JSON payloads
for `GET/PUT /api/labels` are ~3× larger than necessary; a dense segmentation image with 500
polygons × 200 points ships ~1.2 MB where ~400 KB suffices. The PUT body has the same
redundancy, and the server *trusts* the client's pixel data only incidentally (it re-parses
from disk after save).

## Design

Wire type ≠ domain type. Introduce an explicit DTO instead of skip-attributes on the domain
struct (skip-attrs would break `yoda-app`'s internal use of serialized state and make invariants
implicit):

```rust
// yoda-core (so both server and WASM client share it)
#[derive(Serialize, Deserialize)]
pub struct LabelWire {
    pub class_id: u32,
    pub label_type: LabelType,
    pub normalized_coords: Vec<f32>,
    #[serde(default = "default_true")] pub visible: bool, // client→server only meaningful for round-trip
}

impl LabelWire {
    pub fn from_label(l: &LabelObject) -> Self { … }
    pub fn hydrate(self, index: usize, w: u32, h: u32) -> Result<LabelObject, LabelError> { … }
}
```

`hydrate` reuses the existing `parse_bbox` / `parse_polygon` conversion logic — refactor those
two so both the text parser and `hydrate` call shared
`fn bbox_from_normalized(coords, w, h)` / `fn polygon_from_normalized(coords, w, h)` helpers.
`index` is assigned by position server- and client-side, which also removes the client's
ability to send inconsistent `index` values.

## Changes

1. `yoda-web`: `LabelsResponse.labels: Vec<LabelWire>`, `SaveLabelsRequest.labels: Vec<LabelWire>`.
   Handlers hydrate after receive / dehydrate before send. `save_labels` validation comes free:
   `hydrate` errors → `ApiError::bad_request` (today a malformed body can write garbage
   normalized coords to disk — this closes that hole too).
2. `yoda-ui`: mirror structs (lib.rs:332-361) switch to `LabelWire`; hydrate immediately after
   fetch using the response's `width`/`height` (the code already has both). All downstream UI
   code keeps using `LabelObject` unchanged.
3. Version skew: this is a breaking API change. Server and WASM bundle ship together (same
   binary/container), so no compatibility shim is needed — but bump `CARGO_PKG_VERSION` and
   note it in the changelog. The SSR fallback uses server-side types only — unaffected.

## Measurement

Before/after `content-length` of `GET /api/labels` on the densest example image; add the
numbers to the PR description. Re-measure **after** performance/06 gzip: if compression already
brings the delta under ~20%, downgrade this item to "do during the next API break".

## Testing

- Roundtrip property: `LabelObject` → `LabelWire` → `hydrate` == original (for both types;
  float tolerance 1e-4).
- `hydrate` rejects: odd coord counts, <3 polygon points, non-finite floats, coords far outside
  [0,1] (allow small overshoot, e.g. clamp within [-0.05, 1.05], matching YOLO tooling
  leniency — decide and document).
- Update the Axum `save_labels_endpoint_persists_updates` test to the new body shape.

## Risks

- Duplicated struct definitions in `yoda-ui` already drift-prone (LabelsResponse etc.) — this
  is a good moment to move all wire DTOs into a tiny shared `yoda-api` module or into
  `yoda-core::wire`, imported by both sides, eliminating the mirrors.
