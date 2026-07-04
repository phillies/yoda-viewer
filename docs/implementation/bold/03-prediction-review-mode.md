# Prediction vs Ground-Truth Review Mode

> Source: optimizations-and-features.md §7.3 · Effort: L
> Depends on: none of the ML infrastructure (that's the point) — pure file comparison.
> Cheapest of the bold bets relative to impact; recommend building before bold/01.

## Concept

Point YoDa at a second label root containing model predictions in the same mirrored YOLO
layout (`yolo predict --save-txt --save-conf` output). Render GT and predictions together,
compute per-image agreement metrics, and **sort the tree by disagreement** — which is
zero-infrastructure active learning: the most-disagreed images are where label errors and
hard examples live.

## Configuration

`YODA_PREDICTION_BASE_PATH` (optional; feature invisible when unset). Predictions may carry a
trailing confidence per line (`--save-conf`) — extend the parser to tolerate it:
bbox lines with 5 coords and polygon lines with odd count = 2n+1 → last value is confidence.
Implement in `yoda-core` as `parse_yolo_labels_with_conf` sharing the line parser
(coordinate with correctness/01's lossy refactor — do them together; confidence field:
`pub confidence: Option<f32>` on `LabelObject`, serde-skipped when None).

## Server

### Loading

`BackendState` gains `prediction_root: Option<PathBuf>`. Extend `LabelsResponse`
(**additively**): `predictions: Option<Vec<LabelWire>>` loaded via the same
`map_image_to_label_path` against the prediction root.

### Metrics

New module `yoda-core::eval`, pure functions:

```rust
pub struct ImageEval {
    pub matches: Vec<(usize /*gt*/, usize /*pred*/, f32 /*iou*/)>,
    pub missed_gt: Vec<usize>,       // false negatives
    pub spurious_pred: Vec<usize>,   // false positives
    pub class_mismatches: Vec<(usize, usize)>,  // matched geometry, different class
    pub mean_iou: f32,
}
pub fn evaluate(gt: &[LabelObject], pred: &[LabelObject], iou_thresh: f32) -> ImageEval
```

- IoU: bbox trivial; polygon IoU via rasterization at reduced scale (render both to a ~256px
  bitmap and count — `imageproc` or hand-rolled scanline fill; exact polygon clipping is
  overkill). Greedy matching by IoU desc (Hungarian unnecessary at these densities).
- **Dataset-level pass**: a features/07-style background job computing per-image summary
  `(mean_iou, fn_count, fp_count, mismatch_count)` for the whole prediction set → this feeds
  the tree sort. Cache to `.yoda/eval.json` (or the features/11 DB) keyed by label+pred file
  validators.

```
GET /api/eval/status | POST /api/eval/run
GET /api/eval/summary        → per-image metrics map
```

## Client

- **Overlay**: predictions render in a distinct style — same class colors but dashed stroke +
  no fill (GT keeps fills); legend gets a GT/pred line-style key. Toggle "Show predictions"
  in the toolbar. Confidence chip on prediction labels when present.
- **Object panel**: predictions listed in their own collapsible section, each tagged
  `match (IoU 0.87)` / `spurious` / class-mismatch warning icon; matched pairs highlight
  together on hover (hover state keyed by match pair).
- **Tree sort/filter**: sort dropdown in the tree header — "path (default) / disagreement
  desc"; filter chips "has FN", "has FP", "class mismatch" composing with existing filters
  (features/06's generalized predicate).
- **Promote prediction → GT**: accept-button on a spurious-or-better prediction copies it
  into `current_labels` + save (one-click label fixing; this is bold/01's accept flow without
  any model runtime — build the shared `accept proposal` reducer action here first).

## Testing

- `eval` unit tests: hand-computed IoU cases (bbox exact, polygon vs rasterized reference),
  greedy matching ties, empty GT/empty pred edges, class-mismatch detection.
- Parser: confidence-suffix tolerance for both bbox and polygon lines.
- Axum: labels endpoint includes predictions when configured; eval summary job on fixture.
- E2E: enable predictions, assert dashed overlay present, accept one → object count +1.

## Risks

- Polygon rasterized IoU accuracy vs speed — 256px raster gives ±1–2% IoU error; fine for
  ranking. Document the method.
- Confidence-in-file parsing must never leak into normal GT saves (round-trip: confidence is
  never serialized by `serialize_yolo_labels`) — add the regression test.
