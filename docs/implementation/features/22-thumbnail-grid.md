# Thumbnail Grid View

> Source: optimizations-and-features.md §6.6 · Effort: L
> Depends on: performance/05 (blocking pool), performance/06 (cache headers);
> features/11 optional (bookkeeping) · Composes with: class filter, filename search,
> features/08 unlabeled filter

## Design

### Server: thumbnail generation + serving

- `GET /api/thumbnail?image_path=…&size=192` → JPEG (quality ~80), longest edge = size
  (allow-list sizes: {128, 192, 256} to bound cache variants).
- Disk cache: `<label_root>/.yoda/thumbs/<size>/<relative-path>.jpg` (same dot-dir as
  features/11; read-only deployments → platform cache dir fallback, same rule as the DB).
  Validator: regenerate when source mtime+size differ from cached (stash validators in the
  thumb file's own mtime comparison — simplest: regenerate if `thumb.mtime < src.mtime`).
- Generation: `image::open` → `thumbnail(w, h)` (fast integer downscale) in `spawn_blocking`.
  Concurrency cap: a `tokio::sync::Semaphore(num_cpus)` around generation so a fresh grid
  view doesn't stampede the blocking pool with 200 decodes; misses beyond the cap queue.
- Serve with `Cache-Control: private, max-age=3600` + ETag (performance/06 machinery) —
  thumbs are derived, cheap to revalidate.
- **No pre-generation pass** in v1: lazy on-demand keeps startup fast; the grid populates
  progressively (browsers request as `<img>`s enter viewport with `loading="lazy"`).

### Client: grid mode

- Toggle in the tree panel header: list ⇄ grid (persisted via features/14 prefs).
- Grid replaces the tree's *file* portion: current filter + search results as a CSS grid of
  cards (`img loading="lazy"` + filename + unlabeled badge). Folder navigation: keep it flat —
  the grid shows **all matching images across folders** (that's its value: filter-then-skim);
  breadcrumb scoping can come later.
- Virtualization: `loading="lazy"` defers network but 50k `<img>` nodes still hurt the DOM.
  V1: paginate ("Show more", pages of 500). V2: windowed rendering keyed on scroll position —
  only if v1 measurably lags (performance/07 discipline).
- Selection: click card = open image in viewer (switches back to viewer context, grid state
  preserved); the card of the current image is highlighted.
- Overlay-on-thumbnail (render labels into thumbs): tempting, **defer** — it multiplies cache
  invalidation by label edits and bloats scope; a colored class-dot strip on the card
  (from class index, no pixels needed) gives 80% of the skim value free.

## Testing

- Axum: thumbnail request → 200 jpeg with correct max dimension; second request hits cache
  (assert via generation counter or file mtime unchanged); source touched → regenerated;
  path traversal guarded (reuses `resolve_path` ✓ — write the test anyway).
- Semaphore: N concurrent cold requests → ≤ cap decodes in flight (inject a counter).
- E2E: switch to grid, cards render, click card → viewer opens that image.

## Risks

- Disk usage: ~10–30 kB × images × sizes; document, and add `.yoda/thumbs` to the hidden-file
  exclusions (dot-dir already excluded from scans ✓) plus a "Clear thumbnail cache" note in
  docs (endpoint later if asked).
- EXIF orientation: `image` crate does not auto-rotate; JPEGs from cameras may thumb sideways.
  Read the orientation tag (`kamadak-exif`, tiny dep) and rotate at generation — cheap and
  correct; full-size viewer has the same issue but browsers handle EXIF in `<img>` natively,
  thumbs re-encoded lose it. (This is the subtle one — don't skip.)
