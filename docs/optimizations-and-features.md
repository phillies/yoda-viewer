# YoDa Viewer — Codebase Analysis: Optimizations & Feature Ideas

> Generated: 2026-07-04 · Branch: `main` @ `75d1eab`
> Companion to [`docs/next-features.md`](next-features.md) (2026-07-01). Items already covered
> there are only referenced, not repeated — this document focuses on new findings from a fresh
> read of the whole workspace, plus a set of bolder long-term options.

---

## 1. Overall Assessment

The codebase is in good shape. Strengths worth preserving:

- **Clean layering.** `yoda-core` (domain) → `yoda-data` (repository) → `yoda-app` (reducer +
  effects) → `yoda-ui` / `yoda-web` / `yoda-desktop` (frontends). Dependencies point in one
  direction and the `DatasetRepository` trait keeps the filesystem swappable.
- **The reducer pattern** (`apply_action` returning `ActionResult { effects, blocked }`) is a
  solid foundation — it makes undo/redo, keyboard shortcuts, and testing cheap to add later.
- **Good unit test coverage** across parsing, rendering (incl. an insta snapshot), config,
  repository, reducer, and Axum handlers (`tower::ServiceExt::oneshot`).
- **Sensible lints** (`unwrap_used`, `todo`, `dbg_macro` denied; `unsafe_code` forbidden).
- Path traversal on the API is handled correctly (`resolve_path` canonicalizes and checks
  `strip_prefix(root)`), with a test proving it.

The main weaknesses are: a handful of correctness papercuts, per-render cloning in the UI that
will hurt on large datasets, per-edit O(dataset) disk writes, no CI, and a Docker build with no
layer caching.

---

## 2. Correctness Issues (new findings)

### 2.1 SVG text labels don't escape class names — malformed overlay
`render_text_label` in `crates/yoda-core/src/render.rs` interpolates the class name straight
into the SVG `<text>` element. A class named `R&D`, `<5cm`, or anything with `&`/`<` produces
invalid XML — the browser silently refuses to render the *entire* overlay data-URI image, and
the failure looks like "labels stopped drawing". Add an `escape_xml` helper (the web crate
already has `escape_html` — worth moving into `yoda-core` and sharing). One test with a
hostile class name would lock this in.

### 2.2 `marchingAnts` animation is referenced but never defined
The selected-polygon style in `render.rs:125` sets
`animation: marchingAnts 0.6s linear infinite;` but no `@keyframes marchingAnts` exists in
`APP_CSS`, `FALLBACK_CSS`, or the generated SVG. Selection highlight silently degrades to a
static dashed outline. Either define the keyframes inside the SVG (`<style>` block in the
wrapper) or drop the dead property. Note: CSS animations inside an `<img src="data:image/svg…">`
are rasterized-once in some browsers — see §3.2, which would also fix this properly.

### 2.3 Label file writes are not atomic
`write_yolo_labels` uses `fs::write` directly. A crash/power-loss mid-write truncates the
user's label file — and since every class change/delete/draw triggers an immediate save, the
window is hit on every edit. Standard fix: write to `<file>.tmp` in the same directory, then
`fs::rename` over the target. Same applies to `ClassIndex::save_to_disk`.

### 2.4 Whole-file parse failure hides partially-good label files
`parse_yolo_labels` maps any `LabelError` to an empty `Vec` (`unwrap_or_default`). One
malformed line makes an image with 200 valid objects appear *unlabeled* — arguably the worst
possible behavior for a QA tool, because it looks like data loss and a subsequent edit+save
would actually **erase the valid lines on disk**. Recommendation: parse line-by-line,
keep valid objects, and surface skipped-line diagnostics (`LabelsResponse.warnings: Vec<String>`)
in the status bar. This is the highest-value small change in this document.

### 2.5 Desktop app launches even when the backend thread failed
`yoda-desktop/src/main.rs` spawns the Axum server on a thread and logs errors there, but
`dioxus::launch` proceeds regardless — a bad `YODA_IMAGE_BASE_PATH` gives a window full of
failed fetches with the real cause buried in stderr. Build the router *before* spawning the
thread (it's synchronous) and show a friendly config-error screen when it fails.

### 2.6 `>100` class IDs get white in `YoDaConfig` accessors
`load_color_map` pre-fills defaults for class IDs 0–99 only; `get_color_tuple`/`get_color_string`
fall back to white instead of `default_color_for_class(id)`. Most call sites bypass this via
`unwrap_or_else(default_color_for_class)`, but the config accessors are inconsistent with them.
Make the fallback call `default_color_for_class` and delete the 0–100 pre-fill loop entirely.

Also still open (tracked in `next-features.md` §1): stale `AppState.view` vs the JS pan/zoom
controller, zoom-unaware close-zone radius, stale class-index on external edits, and the unused
legacy `/api/tree` routes.

---

## 3. Performance Optimizations

### 3.1 Stop cloning `AppState` on every render — biggest UI win
`App()` in `yoda-ui/src/lib.rs` does `let state_value = app_state();` — a full clone of
`AppState` per render. That clone includes `class_index: HashMap<String, Vec<u32>>`, i.e. **an
entry for every image in the dataset**. On a 100k-image dataset every toolbar toggle clones a
~100k-entry map (plus `current_labels`, `class_map`, …). Fixes, in order of impact:

- Move `class_index` out of `AppState` into its own `Signal`/`use_memo` — the reducer only
  needs it for the filter computation, which already runs inside a `use_memo`.
- Read via `app_state.read()` guards in render code instead of cloning the whole struct.
- Wrap `visible_labels()` and `render_overlay_data_uri(...)` in `use_memo` keyed on the fields
  they actually depend on; today the SVG string + percent-encoding is rebuilt on every render,
  even ones triggered by tree expansion.

### 3.2 Render the overlay as inline SVG instead of a data-URI `<img>`
`render_overlay_data_uri` builds an SVG string, percent-encodes it, and hands the browser a
data URI to decode — three copies of the geometry per frame, and it breaks CSS animation
(§2.2) plus devtools inspectability. Dioxus can render the polygons directly as `svg {}`
elements (the `CanvasOverlay` component already does exactly this for draw-mode). Unifying on
one inline SVG overlay removes the encode/decode round-trip, enables per-element hover/selection
styling, and deletes the parallel hit-area layer.

### 3.3 Trim the label wire format
`LabelObject` serializes `normalized_coords` **and** `pixel_points` **and** `pixel_bbox` — the
latter two are pure functions of the former plus image dimensions, which `LabelsResponse`
already carries. For dense segmentation datasets this triples JSON payloads and save-request
bodies. Send normalized coords only and derive pixels client-side (the code for it already
exists in `yoda-core`, which compiles to WASM). `#[serde(skip)]` on derived fields plus a
`hydrate(width, height)` step would do it.

### 3.4 Per-edit disk work is O(dataset)
Every `PUT /api/labels` rewrites the **entire** `.yoda_class_index.json` (all entries) while
holding the write lock. On a large dataset that's hundreds of ms of serialization + I/O per
class-dropdown change. Options: debounce saves (dirty flag + periodic/shutdown flush), or move
the index to something incrementally updatable (`redb`/SQLite — see §6.4). Also drop the lock
before the disk write: clone-then-write.

### 3.5 Blocking I/O on the async runtime
`image_bytes`, `image_dimensions`, `load_labels`, and `save_labels` all do synchronous
`std::fs` / `image` crate work directly in Axum handlers. One slow disk (NFS-mounted training
data is the *normal* case for this tool) stalls unrelated requests. Wrap repository calls in
`tokio::task::spawn_blocking`, or make the trait async. Low urgency for a single-user local
tool, important the moment it runs on a shared training server.

### 3.6 HTTP caching + compression — cheap wins
- `GET /api/image` streams the whole file with no `Cache-Control`/`ETag`; every image revisit
  re-downloads megabytes. Dataset images are immutable in practice — an
  `ETag` (mtime+size) with `304` handling, or `Cache-Control: max-age` keyed by a content hash
  in the URL, makes prev/next navigation feel instant.
- Add `tower-http` `CompressionLayer` (feature `"compression-gzip"`/`"compression-br"`).
  `/api/tree/flat` and `/api/class-index` are large, highly-compressible JSON — on a 100k-image
  dataset the flat index alone is multiple MB.
- Consider `tower_http::services::ServeFile`-style range support if video ever lands (§7.7).

### 3.7 Flat index scalability ceiling
`/api/tree/flat` ships every node to the client at startup. Fine to ~50k images; beyond that,
initial load and the per-interaction `compute_filtered_rows` full scan (O(nodes) per keystroke)
will drag. Not worth fixing today, but worth *measuring* — add an image-count log line and a
soft warning in the UI beyond a threshold, so you know when to move filtering server-side.

### 3.8 Docker build has no layer caching
`COPY . .` happens before any build, so every source change re-downloads and recompiles all
dependencies **and** `cargo install dioxus-cli` (several minutes). Use `cargo-chef` (or a
`COPY Cargo.toml Cargo.lock crates/*/Cargo.toml` + dummy-main prebuild layer), and pin the
dioxus-cli version (`cargo install dioxus-cli --version 0.7.x --locked`) so builds are
reproducible and cacheable. Also consider a distroless/`debian:trixie-slim` runtime `USER`
other than root.

---

## 4. Code Quality & Maintenance

- **No CI.** `.github/` contains only an agent definition. Add a workflow running
  `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace`, and (on a second job) `dx build` + `docker build`. The repo's
  quality bar is already high; CI just locks it in for PRs like the Copilot ones in history.
- **Duplicated code between `yoda-web` fallback and `yoda-ui`:** `dataset_relative_path` exists
  in both `yoda-data` and `yoda-web`; icon SVGs, tree sorting, and CSS themes are duplicated
  (and have *diverged* — fallback is blue, app is olive). Given §1.8 of `next-features.md`
  questions the SSR fallback's value, consider shrinking the fallback to a plain "build the
  web bundle" instruction page and deleting ~500 lines of parallel HTML rendering.
- **`time = "=0.3.51"` pin** (cookie API mismatch) will silently rot; add a tracking issue or a
  `cargo update` check in CI so the pin gets revisited.
- **Keyboard handling via hidden buttons** works, but Dioxus supports `onkeydown` on the app
  root; migrating removes the `getElementById(...).click()` indirection and makes shortcuts
  testable in Rust. (Shortcut coverage itself: `next-features.md` §2.2.)
- **`web-sys` types in `yoda-ui` non-wasm builds** are cfg-gated correctly, but `reqwest` in the
  desktop webview goes through the network stack for a localhost hop on every interaction —
  acceptable, but worth a comment; a future in-process `AppServices` for desktop would skip
  HTTP entirely (the trait already exists — desktop is *almost* there).
- **E2E smoke test.** Nothing exercises the WASM bundle. One Playwright script (load example
  dataset → click image → toggle bbox → change class → assert label file changed) would catch
  the classic "SSR works, hydration broken" regressions this architecture invites.

---

## 5. Recommended Near-Term Features

`next-features.md` §2 already lists the highest-value UX items (prev/next navigation, shortcut
keys, fit-to-view, tree auto-scroll, last-image persistence, bbox draw mode, filename search,
index progress). All still stand. Additions from this pass:

### 5.1 Unlabeled / label-mismatch surfacing in the tree
The class index already knows which images have zero classes. Badge unlabeled images in the
tree (and add an "unlabeled only" filter chip). Also detect **orphan label files** (label
without image) during the index build — both are everyday QA questions the data already answers.

### 5.2 Save feedback + dirty-state honesty
Saves are fire-and-forget (`run_effects` → spawn). If a save fails the label list still shows
the edited state, silently diverged from disk. Add a save-status indicator (`Saving… / Saved ✓ /
Failed ↻ retry`) driven by the effect result, and re-sync `current_labels` from the
`LabelsResponse` the server already returns from `PUT /api/labels` (it's currently discarded).

### 5.3 Per-image object count + class summary in the tree tooltip
Cheap with `class_index` in hand; hovering a file shows `3× wheel, 1× bumper`. Great for
skimming without opening images.

### 5.4 `POST /api/class-index/rebuild` + `notify` file watcher
Union of `next-features.md` §1.5/§3.4: manual rebuild endpoint first (an afternoon), then a
`notify`-based label-dir watcher that patches the index and (later) pushes updates over SSE.

### 5.5 Read-only deploy flag
`YODA_READ_ONLY=1` to disable the PUT route and hide edit UI. Trivial, and makes it safe to
point the Docker image at a production dataset for browsing — today anyone who can reach the
port can rewrite labels (`0.0.0.0` is the Docker default host). A shared-token option
(`YODA_AUTH_TOKEN`) is the natural second step.

### 5.6 Multi-select + batch reclass / batch delete
(Extends `next-features.md` §3.2.) Shift-click in the object list, "select all of class X",
then one dropdown/delete for the lot. The reducer pattern makes this mostly an `AppAction`
addition (`SelectionSet(Vec<usize>)` + batch variants).

---

## 6. Medium-Term / Structural Features

### 6.1 Undo / Redo (restating — it's the top gap)
Every mutation currently hits disk irreversibly. A bounded history ring of
`(Vec<LabelObject>, description)` snapshots in `AppState`, with `Ctrl+Z`/`Ctrl+Shift+Z`, is
straightforward given the reducer. Per-image history is enough; cross-image undo is a bonus.

### 6.2 Dataset statistics dashboard
A `/stats` view computed from `ClassIndex` + label files: per-class instance & image counts,
class co-occurrence matrix, objects-per-image histogram, polygon-area distribution, per-split
(train/val/test inferred from top-level folders) breakdowns. Detecting class imbalance before
training is the #1 reason people open a dataset viewer. Export as CSV/JSON.

### 6.3 Dataset-wide class operations
Rename, **merge** (reassign all `id=7 → id=3`), delete-class-everywhere, and renumber-compactly —
with a dry-run preview ("would touch 1,243 files") and the YAML `names:` block rewritten to
match. This is painful to do with shell scripts and exactly the kind of bulk mutation a tool
with an index can do safely (write-ahead: back up touched label files to `.yoda_backup/`).

### 6.4 SQLite (or `redb`) as the metadata store
One file next to the labels replaces `.yoda_class_index.json` and unlocks: incremental updates
(§3.4), review states (§7.4), edit history/audit log (§6.1's persistent big brother), saved
filters, and stats caching. `rusqlite` is boring and perfect for this. Keep YOLO `.txt` files
as the single source of truth for geometry — the DB is a rebuildable cache + workflow layer.

### 6.5 Polygon vertex editing & bbox resize handles
(Extends `next-features.md` §3.7.) With inline-SVG overlays (§3.2) each vertex becomes a real
DOM node — drag handles, mid-edge insert (click an edge to add a vertex), vertex delete, and
polygon simplification (Ramer–Douglas–Peucker with a tolerance slider, valuable for
model-generated masks with hundreds of redundant points).

### 6.6 Thumbnail grid view
A gallery mode next to the tree (server-side thumbnail generation, cached to
`.yoda_thumbs/`, served with long-lived cache headers). Scanning 50 thumbnails beats opening
50 images; combine with the class filter for "show me every image with a `person`" browsing.

---

## 7. Bold / Ambitious Options

Ranked roughly by ambition. All are compatible with the current architecture; each names its
enabling tech.

### 7.1 Model-assisted labeling ("pre-annotate with YOLO")
Load an ONNX-exported YOLO model server-side (`ort` crate; `tract` or `candle` as pure-Rust
alternatives) and add "Suggest labels for this image / this folder". Proposals arrive as a
distinct *pending* state (dashed outline, accept/reject per object or per image). Since
Ultralytics exports ONNX in one line, users can bring the very model they're training —
turning YoDa from a viewer into a human-in-the-loop labeling accelerator. This is the single
feature that would most change what the tool *is*.

### 7.2 Click-to-segment with SAM/MobileSAM
One-click object masks: run a small SAM-family encoder/decoder in ONNX (server-side, or even
client-side via WebGPU + `wonnx`/transformers.js-style setup), click an object, get a polygon,
snap it into the existing draw pipeline. Drawing polygons by hand is the most tedious part of
segmentation labeling; this removes ~90% of it. Pairs with §6.5's simplification to keep the
generated polygons small.

### 7.3 Prediction-vs-ground-truth review mode
Point YoDa at a second "labels" root (`YODA_PREDICTION_BASE_PATH`, same mirrored layout — YOLO
`predict --save-txt` output). Render GT and predictions in contrasting styles, compute
per-image IoU / missed / spurious counts, and sort the tree by disagreement. That sort order
*is* active learning at zero model-infrastructure cost: "show me the images my model gets most
wrong" is the fastest route to both label errors and hard examples.

### 7.4 Multi-user review workflow
Per-image status (`unreviewed / approved / needs-fix`), reviewer notes, and an audit log —
stored in the SQLite layer (§6.4). Add SSE/WebSocket presence ("Anna is viewing train/img_042")
and last-writer-wins conflict warnings. Doesn't need accounts to be useful: a name-in-a-cookie
plus statuses covers a two-person labeling team, which is the realistic deployment.

### 7.5 Embedding-based dataset exploration & dedup
Compute image embeddings once (CLIP-family ONNX, or even trivial perceptual hashes as v1),
store vectors in SQLite. Unlocks: near-duplicate detection across train/val (the classic
leakage bug), "find similar images" from any image, and a 2-D UMAP scatter of the dataset
colored by class for spotting clusters and outliers. Perceptual-hash dedup alone
(`image_hasher` crate, no ML) is a weekend project with outsized payoff.

### 7.6 Format bridges: COCO / VOC / CVAT import-export
Read/write COCO JSON and Pascal VOC alongside YOLO txt. Import makes YoDa the *viewer of
record* for datasets sourced anywhere; export makes it a converter people arrive for and stay.
The clean `yoda-core` label model is 80% of the work already done — each format is a codec
module with roundtrip tests.

### 7.7 Video & sequence annotation
Treat `.mp4`/frame-sequence folders as first-class: frame scrubber, label copy-forward
("propagate to next N frames"), and simple linear interpolation of boxes between keyframes.
Requires the range-request serving from §3.6 and a frame-extraction step (ffmpeg sidecar or
`ffmpeg-next`). Big lift, but tracking datasets are where hand-labeling pain peaks.

### 7.8 Single-binary distribution + `yoda serve` CLI
Embed the built WASM bundle in the server binary (`rust-embed`/`include_dir`) so
`cargo install yoda-viewer && yoda serve --images ./images --labels ./labels` works with zero
asset directories — eliminating the SSR-fallback confusion class of issues entirely. CLI args
via `clap` as a friendlier front door than env vars (keep env for Docker). Then publish to
crates.io + a Homebrew tap; this is the highest-leverage *adoption* feature in the list.

---

## 8. Suggested Priority Order

| Tier | Items | Rationale |
|------|-------|-----------|
| **Now (correctness)** | §2.4 partial parse (data-loss risk), §2.3 atomic writes, §2.1 XML escaping, §5.2 save feedback | Small diffs; all protect user data or surface silent failures |
| **Now (infra)** | §4 CI workflow, §3.8 Docker caching | One-time cost, protects everything after |
| **Next (perf)** | §3.1 state-clone fix, §3.6 caching+compression, §3.2 inline SVG overlay | Felt on every interaction; unblock §6.5 |
| **Next (UX)** | `next-features.md` §2 backlog + §5.1 unlabeled surfacing, §5.5 read-only flag | Highest value-per-line UX items |
| **Then** | §6.1 undo/redo, §6.2 stats, §6.4 SQLite, §6.3 class ops, §6.6 thumbnails | Structural; each enables later tiers |
| **Bold bets** | §7.8 single binary → §7.5 dedup (phash v1) → §7.3 pred-vs-GT → §7.1/§7.2 model-assisted | Ordered by cost; 7.8 and phash-dedup are shockingly cheap for their impact |
