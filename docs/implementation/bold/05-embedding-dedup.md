# Duplicate Detection & Embedding-Based Exploration

> Source: optimizations-and-features.md §7.5 · Effort: S (phash tier!) → XL (embeddings)
> Depends on: features/11 (vector/hash storage), features/07 (job progress) ·
> bold/01's ORT setup only needed for Tier 2+

## Why this ordering: Tier 1 is a weekend, catches the classic leakage bug

### Tier 1 — perceptual-hash dedup (no ML, ship first)

- `image_hasher` crate (maintained fork of img_hash): 64-bit dHash/pHash per image.
- Compute during a features/07-style background job (decode → hash; reuse thumbnail decodes
  if features/22 landed — hash the 192px thumb, quality is sufficient and it's ~free);
  store in the DB: `image.phash INTEGER`.
- Near-duplicate = Hamming distance ≤ threshold (default 6, exposed). Exact-file dupes:
  also store `(size, blake3-of-first-64KB)` — catches copies before decoding.
- Grouping: at ≤ ~200k images, brute-force pairwise on 64-bit ints with early-exit popcount
  is ~seconds in Rust — no index structure needed v1 (BK-tree if measurements disagree).
- **The killer report**: duplicate groups that span top-level folders (train ∩ val) —
  train/val leakage, surfaced by name.

```
POST /api/dedup/run     GET /api/dedup/status
GET  /api/dedup/groups  → [{hash_dist, members: [paths…], spans_splits: bool}]
```

- UI: "Duplicates" section on the stats page (features/20): group cards with thumbnails
  side-by-side (features/22), leakage groups pinned to top with a red badge. Click →
  open image. No auto-deletion — YoDa reports; the user acts (deleting images is out of
  scope until asked; at most: copy group list as text/CSV).

### Tier 2 — embeddings + similarity search

- CLIP-family image encoder via ONNX in `yoda-infer` (bold/01's crate; e.g. quantized
  ViT-B/32 image tower, ~40 MB): image → 512-d f32 vector, background job, store as BLOB in
  DB (`embedding` column or table).
- "Find similar": from any open image, brute-force cosine over all vectors —
  100k × 512 dot products ≈ tens of ms with simple SIMD-friendly code; no ANN index until
  proven necessary (usearch/hnsw as escape hatch).
- UI: "Similar images" strip in the right panel (thumbnail row, click to open). Also:
  "label the ones like this" = similarity + unlabeled filter — a genuinely powerful
  labeling loop with no extra machinery.

### Tier 3 — 2-D map (defer until 2 proves value)

UMAP/t-SNE projection (run in Rust via `annembed`, or offline escape hatch: export
embeddings as .npy for a notebook) → scatter view colored by class/status; lasso-select →
filter. Big UI lift (canvas rendering, zoom/brush); only justified by real demand.

## Testing

- Tier 1: unit tests with generated images (same image re-encoded → distance 0–2; distinct →
  large), split-spanning detection, hash job resumability (validators: skip already-hashed).
- Tier 2: golden cosine-similarity test with a pinned tiny model; top-k correctness vs
  brute-force reference (they're the same in v1 — test the SIMD path if added).

## Risks

- pHash false positives on flat/synthetic images (renders, documents) — the threshold is
  exposed and groups show thumbnails; humans adjudicate.
- Embedding model licensing/distribution — same policy as bold/02: env-var paths + documented
  sources, nothing bundled.
