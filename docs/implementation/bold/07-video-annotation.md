# Video & Sequence Annotation

> Source: optimizations-and-features.md §7.7 · Effort: XXL — largest item in the roadmap
> Depends on: features/01/02 (paging UX), features/19 (undo), features/16 (vertex edit),
> performance/06 (caching) · Recommend the sequence tier only unless tracking datasets are a
> confirmed use case.

## Two tiers — the first is 20% of the cost and most of the value

### Tier 1: frame-sequence folders (no video decoding at all)

Datasets extracted to frames (`seq01/000001.jpg …`) already work in YoDa; what's missing is
sequence *ergonomics*:

- **Sequence detection**: a folder whose images match a numeric-suffix pattern (≥ N=20 files,
  common stem/padding) is flagged `is_sequence` in the flat index (detection in
  `scan_dataset_tree`, pure name heuristics, unit-testable).
- **Scrubber UI**: when the selected image belongs to a sequence, show a timeline bar under
  the viewport (frame slider + play/pause at ~5–10 fps via preloaded neighbors + step
  buttons). Playback = timed selection advance reusing the features/01 pipeline; prefetch
  next ~10 frames (`<link rel=prefetch>`-style hidden `<img>`s or fetch+cache — with
  performance/06 ETags this is cheap).
- **Label copy-forward**: action "copy labels to next frame" (`AppAction::CopyLabelsTo
  {path}` → server-side copy of current labels to the target's label file + index patch);
  hold-to-repeat = annotate slow scenes fast. With undo (features/19) this is safe.
- **Linear interpolation**: select object, mark keyframe K1 at frame i, jump to frame j,
  move/edit it, "interpolate" → server writes frames i+1..j-1 with lerped geometry
  (bbox: corner lerp; polygon: vertex lerp when counts match, else refuse with message).
  Pure functions in `yoda-core::interp`, heavily unit-tested.

Tier 1 needs zero new decoding infrastructure and serves every tracking-dataset workflow
that ships as frames (most do).

### Tier 2: native video files (only on demand)

- Decode: `ffmpeg` **sidecar binary** (spawn, parse) over linking `ffmpeg-next` — licensing
  and build complexity of linking libav on 3 platforms is not worth it; a documented
  "ffmpeg on PATH" requirement is honest. Feature-gated.
- Serving: extract-on-demand frame endpoint
  (`GET /api/video/frame?path=…&index=…` → JPEG, disk-cached like thumbnails) + a
  frame-count/fps probe endpoint (`ffprobe`).
- Labels: map to a virtual frame path (`video.mp4/000123.txt` mirrored layout) so the entire
  existing label pipeline works unchanged — the label root simply contains a directory named
  after the video. This mapping trick is the key design move: everything downstream
  (index, filters, review states) works on virtual frames for free.
- Seeking accuracy (keyframe-only fast seek vs exact): exact (`-ss` after `-i`) is slow on
  long videos; cache extracted frames aggressively; extract in GOP-sized batches.

## Testing

- Tier 1: sequence-detection heuristics table tests; interpolation math (lerp endpoints,
  count-mismatch refusal); copy-forward Axum test; scrubber E2E (arrow through a fixture
  sequence).
- Tier 2: gated integration tests requiring ffmpeg (skip-if-absent), frame extraction
  determinism, virtual-path label roundtrip.

## Risks

- Tier 2's cache can be huge (frames × videos) — same `.yoda/` cache-dir policy + documented
  clearing as features/22; add size accounting to the status endpoint from day one.
- Scope: object *tracking* (auto-follow across frames) is explicitly out — that's bold/01/02
  territory applied per-frame; interpolation + copy-forward is the honest manual-tool
  boundary.
