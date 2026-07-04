# Model-Assisted Labeling (ONNX YOLO pre-annotation)

> Source: optimizations-and-features.md §7.1 · Effort: XL
> Depends on: features/09 (save pipeline), features/19 (undo), features/13 (batch
> accept/reject reuses selection UI) · The feature that changes what YoDa *is*.

## Concept

User points YoDa at an ONNX-exported YOLO model (`yolo export format=onnx` — one line for
Ultralytics users, and they can bring the very checkpoint they're training). YoDa runs
inference on demand and presents proposals as *pending* labels the user accepts or rejects.
Human-in-the-loop labeling accelerator.

## Architecture

### Inference backend: `ort` crate (ONNX Runtime)

- `ort` = mature bindings, CPU-first (`download` strategy binaries or system lib), optional
  CUDA/CoreML execution providers later. Alternatives (`tract`, `candle`) are pure-Rust but
  cover fewer exported graphs; YOLO ONNX exports are ort's bread and butter.
- **Feature-gate everything**: `yoda-infer` new crate + `--features infer` on `yoda-web`, so
  the base build never pays the dependency. Docker: second image tag (`yoda-viewer:infer`).

### `yoda-infer` crate

```rust
pub struct Detector { session: ort::Session, input_size: u32, … }
impl Detector {
    pub fn load(path: &Path, config: DetectorConfig) -> Result<Self>;
    pub fn detect(&self, image: &DynamicImage) -> Result<Vec<Proposal>>;
}
pub struct Proposal { class_id: u32, confidence: f32, geometry: ProposalGeometry }
pub enum ProposalGeometry { Bbox([f32;4] /*normalized cx,cy,w,h*/), Polygon(Vec<f32>) }
```

Implementation content (the real work):
- Preprocess: letterbox resize to model input (typ. 640), NCHW f32 normalize.
- Postprocess v8/v11 detect heads: transpose output, confidence filter, class argmax,
  **NMS** (implement classic greedy IoU NMS — no dep needed), un-letterbox coords, normalize.
- Segmentation heads: proto-mask matmul + crop + threshold → binary mask → **contour trace**
  (`imageproc::contours`) → polygon → RDP simplify (shared with features/16).
- Config: confidence threshold (default 0.25), IoU threshold, class-id offset mapping
  (model classes vs dataset YAML may disagree — expose a remap table in config, default
  identity, warn when model class count ≠ dataset class count).

### Server surface

```
POST /api/infer?image_path=…          → proposals for one image (sync, seconds)
POST /api/infer/batch {paths|filter}  → job; progress via features/07-style status
GET  /api/infer/status
```

Model configured by env (`YODA_MODEL_ONNX=…`) or uploaded/selected path via a settings
endpoint (env-only v1). One `Detector` in `BackendState` behind `OnceCell`, inference calls in
`spawn_blocking` with a semaphore(1..2) — ORT sessions are internally threaded.

### Proposal lifecycle (client)

- Proposals arrive as overlay objects in a distinct visual state: dashed outline + confidence
  chip; **not** in `current_labels`, in a parallel `proposals: Vec<Proposal>` signal.
- Accept (per object): converts to a real label (`hydrate` from normalized — performance/03's
  machinery) → append + save. Accept-all-above-slider (confidence slider in a proposals
  panel). Reject removes from the list; nothing touches disk.
- Batch mode: run over the filtered image set; images gain a "has proposals" badge; review
  becomes: open image → accept/reject → next (features/01 keyboard flow). Proposals persist
  server-side in memory per job (or features/11 DB table `proposal` for resumable sessions —
  v2).

## Order of implementation

1. `yoda-infer` with bbox-only v8 detect + unit tests against a tiny reference model
   (export yolov8n at 64px? — commit a fixture ONNX ≤5 MB or download-on-test with checksum).
2. Single-image endpoint + proposal overlay + accept/reject.
3. Segmentation head support.
4. Batch jobs + review flow.

## Testing

- Golden-file tests: fixture image + fixture model → assert proposal count/classes/IoU vs
  recorded expectations (tolerances; pin ort version).
- NMS/letterbox unit tests (pure functions, easy).
- E2E behind feature flag: accept a proposal → label file contains it.

## Risks

- ONNX Runtime binary distribution (per-OS ~tens of MB; `ort`'s `download` feature handles
  it, but offline/air-gapped installs need the system-lib path documented).
- Model zoo drift: pin to "Ultralytics v8/v11 export layouts"; reject unknown output shapes
  with a clear error listing expected dims. Do not attempt universal YOLO-family support.
- Class-mapping mistakes silently create wrong labels — the remap-table warning above is a
  hard requirement, not polish.
