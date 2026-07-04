# Implementation Documents — Index

> Generated: 2026-07-04 · One design doc per feature/issue from
> [`../next-features.md`](../next-features.md) (2026-07-01) and
> [`../optimizations-and-features.md`](../optimizations-and-features.md) (2026-07-04).
> Duplicated items between those two documents are merged here (each doc's header names its
> sources). Effort scale: XS (<½ day) · S (≤1 day) · M (days) · L (~1–2 weeks) · XL+ (more).

## Correctness (`correctness/`)

| Doc | Title | Effort | Sources |
|-----|-------|--------|---------|
| [01](correctness/01-partial-label-parse.md) | Partial label parsing (data-loss risk) | S | opt §2.4 |
| [02](correctness/02-atomic-file-writes.md) | Atomic writes for labels + index cache | S | opt §2.3 |
| [03](correctness/03-svg-text-escaping.md) | XML-escape class names in SVG | XS | opt §2.1 |
| [04](correctness/04-selection-animation.md) | Fix dead `marchingAnts` animation | XS | opt §2.2 |
| [05](correctness/05-desktop-startup-errors.md) | Desktop startup error screen | S | opt §2.5 |
| [06](correctness/06-color-fallback-consistency.md) | Color fallback for class IDs ≥100 | XS | opt §2.6 |
| [07](correctness/07-view-state-sync.md) | View-state source of truth + always-available reset | M | nf §1.1, §1.3, §2.3 |
| [08](correctness/08-close-zone-zoom-scaling.md) | Zoom-aware polygon close zone | S | nf §1.2 |
| [09](correctness/09-filter-unknown-classes.md) | Filter chips for label-only classes | S | nf §1.6 |

## Performance (`performance/`)

| Doc | Title | Effort | Sources |
|-----|-------|--------|---------|
| [01](performance/01-appstate-clone-reduction.md) | Stop cloning AppState per render | M | opt §3.1 |
| [02](performance/02-inline-svg-overlay.md) | Inline SVG overlay (drop data-URI img) | M–L | opt §3.2 |
| [03](performance/03-label-wire-format.md) | Trim label wire format | M | opt §3.3 |
| [04](performance/04-class-index-write-amplification.md) | Debounced class-index persistence | S–M | opt §3.4 |
| [05](performance/05-async-blocking-io.md) | Blocking I/O off the async runtime | M | opt §3.5 |
| [06](performance/06-http-caching-compression.md) | ETag/304 + gzip/brotli | S | opt §3.6 |
| [07](performance/07-flat-index-scalability.md) | Flat-index scalability: measure + guard | S | opt §3.7 |
| [08](performance/08-docker-layer-caching.md) | Docker caching, pinning, non-root | S | opt §3.8 |

## Infrastructure (`infra/`)

| Doc | Title | Effort | Sources |
|-----|-------|--------|---------|
| [01](infra/01-ci-pipeline.md) | CI pipeline (fmt/clippy/test/dx/docker + pin watchdog) | S | opt §4, nf §3.6 |
| [02](infra/02-ssr-fallback-simplification.md) | Shrink SSR fallback, remove legacy tree routes | M | nf §1.7, §1.8, opt §4 |
| [03](infra/03-e2e-smoke-test.md) | Playwright E2E smoke test | M | opt §4 |

## Features (`features/`)

| Doc | Title | Effort | Sources |
|-----|-------|--------|---------|
| [01](features/01-prev-next-navigation.md) | Prev/next image navigation | S | nf §2.1 |
| [02](features/02-keyboard-shortcuts.md) | Full keyboard shortcut map | M | nf §1.4, §2.2 |
| [03](features/03-tree-autoscroll.md) | Tree auto-scroll/expand to selection | S | nf §2.4 |
| [04](features/04-last-image-persistence.md) | Last-image persistence (URL + localStorage) | S | nf §2.5 |
| [05](features/05-bbox-draw-mode.md) | BBox draw mode (drag rectangle) | M | nf §2.6 |
| [06](features/06-filename-search.md) | Filename search in tree | S | nf §2.7 |
| [07](features/07-index-build-progress.md) | Non-blocking index build + progress | M | nf §2.8 |
| [08](features/08-unlabeled-surfacing.md) | Unlabeled images + orphan labels | S–M | opt §5.1 |
| [09](features/09-save-status-feedback.md) | Save feedback + state re-sync | S–M | opt §5.2 |
| [10](features/10-class-index-refresh.md) | Index rebuild endpoint + file watcher | S+M | nf §1.5, §3.4, opt §5.4 |
| [11](features/11-sqlite-metadata-store.md) | SQLite metadata store | L | opt §6.4 |
| [12](features/12-read-only-mode.md) | Read-only mode + shared token | S+S | opt §5.5 |
| [13](features/13-multi-select-batch-ops.md) | Multi-select + batch reclass/delete | M | opt §5.6, nf §3.2 |
| [14](features/14-visibility-persistence.md) | Persist visibility/display prefs | S | nf §1.9 |
| [15](features/15-tree-tooltips.md) | Per-image class tooltips | XS–S | opt §5.3 |
| [16](features/16-vertex-editing.md) | Vertex editing + bbox handles + simplify | L | opt §6.5, nf §3.7 |
| [17](features/17-cors-support.md) | CORS support | XS–S | nf §3.5 |
| [18](features/18-tiff-support.md) | TIFF support (with transcoding) | S | nf §3.9 |
| [19](features/19-undo-redo.md) | Undo/redo | M | opt §6.1, nf §3.1 |
| [20](features/20-stats-dashboard.md) | Stats dashboard + CSV export | L | opt §6.2, nf §3.3, §3.8 |
| [21](features/21-class-operations.md) | Dataset-wide class rename/merge/delete | L | opt §6.3 |
| [22](features/22-thumbnail-grid.md) | Thumbnail grid view | L | opt §6.6 |
| [23](features/23-desktop-packaging.md) | Desktop packaging + first-run UX | L | nf §3.10 |

## Bold bets (`bold/`)

| Doc | Title | Effort | Sources |
|-----|-------|--------|---------|
| [01](bold/01-model-assisted-labeling.md) | ONNX YOLO pre-annotation | XL | opt §7.1 |
| [02](bold/02-sam-click-to-segment.md) | SAM click-to-segment | XL | opt §7.2 |
| [03](bold/03-prediction-review-mode.md) | Prediction vs GT review | L | opt §7.3 |
| [04](bold/04-multi-user-review.md) | Multi-user review workflow | XL | opt §7.4 |
| [05](bold/05-embedding-dedup.md) | Dedup (phash) + embeddings | S→XL | opt §7.5 |
| [06](bold/06-format-bridges.md) | COCO/VOC import & export | L | opt §7.6 |
| [07](bold/07-video-annotation.md) | Video & sequence annotation | XXL | opt §7.7 |
| [08](bold/08-single-binary-cli.md) | Single binary + `yoda` CLI | M | opt §7.8 |

## Dependency highlights (read before sequencing)

```
infra/01 (CI) ─ before everything
correctness/01,02,03 ─ independent, do first
performance/01 ─ before features touching UI state (06, 08, 14…)
performance/02 (inline SVG) ─ hard prerequisite for features/16 (vertex editing)
correctness/07 (view sync) → correctness/08 (close zone) → features/16 handle sizing
features/07 (progress machinery) → features/10 (rebuild), bold jobs (eval, dedup, infer)
features/04 (storage helper + dataset id) → features/14
features/09 (save pipeline) → features/13, features/19; features/19 → safe features/13/16/21
features/11 (SQLite) → features/20 tier2, features/22 bookkeeping, bold/04, bold/05
features/12 (read-only) ─ before any networked deployment; pairs with features/17 (CORS)
bold/01 (yoda-infer) → bold/02 (SAM); bold/03 needs none of it — cheapest bold bet
bold/08 coordinates with infra/02 (both touch the no-assets state)
```

Suggested starting sequence (mirrors optimizations-and-features.md §8):
**infra/01 → correctness/01+02+03 → features/09 → performance/08+06 → performance/01 →
features/01+02+04** — after which the codebase is safe, fast, CI-guarded, and pleasant to
review datasets in; everything else sequences by appetite.
