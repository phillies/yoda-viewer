# Click-to-Segment (SAM/MobileSAM assisted polygon drawing)

> Source: optimizations-and-features.md §7.2 · Effort: XL
> Depends on: bold/01 (shares `yoda-infer`, ORT setup, contour→polygon pipeline — build 01
> first), features/16 (vertex editing to fix up results), correctness/08 (screen-space UX)

## Concept

In a new "Smart select" draw mode, the user clicks a point on an object; a SAM-family model
returns a mask; YoDa converts it to a polygon and drops it into the normal draw/commit flow
(class = current `drawing_class_id`). Positive/negative refinement clicks improve the mask
before committing. Removes ~90% of manual polygon clicking.

## Model choice

MobileSAM (or EfficientSAM/EdgeSAM) over full SAM: encoder small enough for CPU
(~40 ms–1 s/image vs ~10 s), quality sufficient for annotation-with-cleanup. Architecture is
two-part and that structure drives the design:

- **Image encoder**: heavy, runs **once per image** → embedding (~256×64×64).
- **Mask decoder**: tiny (~10 ms), runs per click with (embedding + prompt points).

## Architecture (server-side, v1)

Client-side WebGPU decoding is the flashy path; skip it (WASM+WebGPU ORT maturity, asset
size, desktop-webview parity problems). Server round-trip per click at ~10 ms decode +
network is fine for interaction.

### `yoda-infer` additions

```rust
pub struct SamEncoder { … }   // image → Embedding
pub struct SamDecoder { … }   // (Embedding, Vec<PromptPoint>) → Mask
pub struct PromptPoint { x: f32, y: f32, positive: bool }  // normalized coords
```

- Embedding cache: `HashMap<image_path, Arc<Embedding>>` LRU (cap ~8; embeddings are ~4 MB).
  Encode lazily on first smart-click for an image; **pre-encode on image open** when the
  feature is enabled (background task) so the first click is instant.
- Mask → polygon: threshold → largest connected component → contour trace → RDP simplify
  (pipeline shared with bold/01 segmentation; factor into `yoda-infer::maskpoly`).

### API

```
POST /api/sam/encode?image_path=…        → 202/200 {encoded: true}    (idempotent, warms cache)
POST /api/sam/mask   {image_path, points: [{x,y,positive}…]}
     → {polygon: [f32…], score: f32}
```

Stateless per request apart from the embedding cache — refinement resends all prompt points
(they're tiny).

### Client UX

- Toolbar (unlocked): "✨ Smart" mode beside Draw Polygon / Draw BBox (features/05's mode
  enum grows a `SmartSelect` variant).
- Click = positive point; Alt/right-click = negative point. After each click the returned
  polygon renders as a live preview (same styling as draw-mode committed polygon); prompt
  points render as +/− markers.
- `Enter` commits (existing `FinishDrawing` path with the polygon's points — route through
  `create_label_from_pixels`), `Escape` clears prompts, class dropdown as in draw mode.
- Latency handling: debounce rapid clicks is unnecessary (each click is deliberate); show a
  subtle spinner on the preview while the request is in flight; stale-response guard via a
  request counter (same generation pattern as features/09).

## Model distribution

Two ONNX files (`YODA_SAM_ENCODER`, `YODA_SAM_DECODER` env vars). Don't bundle; document
download sources + checksums. Feature-gated with the same `infer` feature as bold/01.

## Testing

- `yoda-infer`: golden tests with fixture image + pinned MobileSAM export — click at known
  object center → polygon IoU vs recorded reference ≥ 0.9; negative-point refinement shrinks
  mask (assert area decrease).
- maskpoly unit tests: synthetic masks (circle, donut → outer contour picked, two blobs →
  largest picked).
- E2E (feature-gated): smart-click + Enter → object count +1.

## Risks

- Embedding cache memory on rapid browsing — LRU cap + metrics log line.
- Multi-instance ambiguity (click selects the whole horse when you wanted the saddle):
  SAM's multimask output — take highest-score mask v1; expose the 3 candidates as
  Tab-to-cycle in v2 (design the response shape as `masks: [ … ]` from day one).
- This feature's UX lives or dies on features/16 (fixing edges) being done first.
