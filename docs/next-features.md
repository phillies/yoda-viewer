# YoDa — Issues & Potential Next Features

> Generated: 2026-07-01  
> Branch context: `feat/class-filter` (commit `8dc3f0c`)

---

## 1. Known Issues

### 1.1 View state divergence (`AppState.view` vs. JS controller)

`AppState` tracks `ViewTransform { zoom, pan_x, pan_y }` and the reducer handles `SetZoom`, `ZoomBy`, `SetPan`, `PanBy`, and `ResetView` actions. However, the JS `PanZoomController` is fully authoritative at runtime — the Rust state is never read back by the JS side and the JS state is never pushed into `AppState`. This means view-state-dependent logic in Rust (e.g. anything that needs to know current zoom for hit-testing or close-zone scaling) would silently use stale values.

**Mitigation options:** Remove the view fields from `AppState` entirely and treat them as JS-only, or add a `data-zoom` sync path from the JS controller back to Dioxus.

### 1.2 Polygon close-zone radius is zoom-unaware

`CLOSE_ZONE_RADIUS = 18.0` is expressed in image-pixel space. The SVG overlay scales with the image, but the close zone hit area does not adapt to the current zoom level, making it unusably small at low zoom and unnecessarily large at high zoom when the user is trying to click a vertex.

### 1.3 Double-click view reset is disabled in unlocked mode

The JS handler skips view reset on double-click when `data-edit-mode === 'unlocked'`, to avoid accidental resets during label interaction. There is currently no alternative gesture or keyboard shortcut to reset the view when editing is unlocked, leaving the user unable to re-center without switching back to locked mode first.

### 1.4 Keyboard shortcuts limited to Delete / Escape / Enter

`DRAW_SCRIPT` only wires up three keys. The Python version supported `M` (mask toggle), `B` (bbox toggle), `I` (class ID), `N` (class name), `F` (fit-to-view), `E`/`D` (edit/draw mode switch). None of these are implemented in the Rust version, requiring toolbar clicks for every display toggle.

### 1.5 Class index cache becomes stale on external edits

The `.yoda_class_index.json` cache is built once at startup and patched only via `POST /api/labels`. If a user edits label files with an external tool while the server is running, the cached class-to-image mapping silently drifts. There is no file watcher, no cache invalidation endpoint, and no staleness indicator in the UI.

### 1.6 `all_class_ids` is fetched but never used

`ClassIndexResponse.all_class_ids` is fetched at startup and annotated `#[allow(dead_code)]` on the client side. The filter bar derives class chips from `class_map` instead, which only includes classes present in the YAML — classes that appear in label files but are absent from the YAML are invisible to the filter.

### 1.7 Unused legacy tree routes

`GET /api/tree` and `GET /api/tree/children` exist on the Axum router but the Dioxus UI exclusively uses `GET /api/tree/flat`. The legacy routes add surface area without being exercised. The older `TreeNode` recursive format they return also isn't tested in the context of the current UI.

### 1.8 SSR fallback viewer is fully read-only with no pan/zoom

When `dx build` hasn't been run, users land on the server-rendered HTML fallback. It has no interactive pan/zoom, no editing, and no navigation between images. This is the current default experience for `cargo run`.

### 1.9 Object and class visibility state is ephemeral

Per-object visibility (`SetObjectVisibility`) and per-class visibility (`SetClassVisibility`) live only in client-side `AppState`. A page reload resets all visibility to fully visible. This can be disorienting when reviewing large label sets across many images.

---

## 2. Missing Short-Term Features

### 2.1 Previous / Next image navigation

The `FlatIndex` already provides an ordered flat list of images. Prev/Next buttons in the toolbar (or Arrow-Left / Arrow-Right keyboard shortcuts) would enable fast sequential review — the most common workflow for dataset inspection and QA. The `AppAction` enum has no `NavigatePrev` / `NavigateNext` yet.

### 2.2 Keyboard shortcuts for display toggles and mode switches

Wire the following shortcuts to their corresponding `AppAction` dispatches via `DRAW_SCRIPT` (hidden button pattern already in place):

| Key | Action |
|-----|--------|
| `M` | `ToggleSegmask` |
| `B` | `ToggleBbox` |
| `I` | `ToggleClassId` |
| `N` | `ToggleClassName` |
| `F` | `ResetView` (fit / reset) |
| `E` | Switch to Edit interaction mode |
| `D` | Switch to Draw interaction mode |
| `→` / `←` | Next / Prev image (see §2.1) |

### 2.3 Fit-to-view / reset view always accessible

`ResetView` should be triggerable regardless of lock state. A dedicated "Fit" toolbar button and the `F` shortcut (§2.2) would cover this. The JS controller already has a `resetView()` function — it just needs to be called from more paths.

### 2.4 Tree auto-scroll to selected image

When an image is selected from the object list, status bar, or a keyboard navigation shortcut, the tree panel does not scroll to reveal the selected row. Implementing auto-scroll on `selected_image_path` change would keep the tree in sync with the viewport.

### 2.5 Last-opened image persistence

The parity checklist marks this as required. `localStorage` or a `?image_path=` query parameter can restore the last viewed image on reload. The SSR fallback already accepts `?image_path=` — the same mechanism could be used by the SPA.

### 2.6 Bounding box draw mode

The draw toolbar only supports polygon creation. A dedicated "Draw BBox" mode should let the user drag a rectangle rather than click individual vertices. The `LabelType::Bbox` parsing and rendering already exists; only the creation path (dragging a rect, computing `cx/cy/w/h`, calling `create_label_from_pixels`) is missing.

### 2.7 File name search / filter in tree

A text input above the tree that filters visible nodes by filename substring would complement the existing class filter. The filtered-row computation pattern from `compute_filtered_rows` can be extended to also check `node.name.contains(query)`.

### 2.8 Progress indicator during first-run class index build

First-run index build takes ~35 s on the example dataset. The UI shows a loading message only after the browser connects, not during the build. A server-side progress log line count and a `/api/class-index/status` endpoint returning `{ built: bool, progress: f32 }` could feed a progress bar in the tree panel.

---

## 3. Medium-to-Long-Term Feature Ideas

### 3.1 Undo / Redo for label edits

Label mutations (delete, class change, new polygon) are currently irreversible within a session. An undo stack of `AppState` snapshots (or action-based command pattern) would make destructive edits safe. Given the reducer pattern already in place this is straightforward to add as a history ring in `AppState`.

### 3.2 Multi-object selection and batch class reassignment

Selecting multiple objects (shift-click, or select-all within a class) and reassigning them to a different class in one action would speed up relabeling workflows on large datasets.

### 3.3 Dataset statistics panel

A dedicated view showing per-class image counts, annotation counts, train/val/test split breakdowns, and class co-occurrence could be computed from the existing `ClassIndex`. This is useful for detecting dataset imbalances before training.

### 3.4 Class index force-refresh endpoint

`POST /api/class-index/rebuild` (or a `?force=true` query on `GET /api/class-index`) would invalidate and rebuild the cache without requiring a server restart, making external label edits visible immediately.

### 3.5 CORS support for networked deployments

Running YoDa on a remote machine (e.g. a training server) and accessing it from a local browser requires CORS headers on the API responses. `tower-http::cors::CorsLayer` can be added to the Axum router with a configurable allowed-origins setting driven by `YODA_ALLOWED_ORIGINS`.

### 3.6 `dx build` workflow documentation and CI step

The WASM bundle (`dx build --release`) is not part of any documented build step. Adding a `scripts/build-web.ps1` and noting the `dx build` prerequisite in `README.md` would eliminate the confusion caused by always landing in SSR fallback mode.

### 3.7 Polygon vertex editing

After a polygon is drawn, individual vertices cannot be dragged to adjust them. An edit-mode overlay that renders draggable vertex handles would make fine-grained corrections possible without having to delete and redraw.

### 3.8 Export / copy label statistics

A button to download a CSV or JSON summary of the current filtered image set (image path + class IDs per image) would make it easy to cross-reference YoDa findings with training logs.

### 3.9 TIFF image support

Some datasets ship `.tif` / `.tiff` images. The `image` crate supports TIFF natively; only the `IMAGE_EXTENSIONS` constant in `yoda-data` needs to be extended.

### 3.10 Desktop packaging (yoda-desktop)

`yoda-desktop` is currently a thin stub. Packaging it as a signed installer (NSIS / WiX on Windows, .deb / AppImage on Linux) with auto-port selection and a system-tray icon for the server URL would improve the out-of-box experience on machines without Rust toolchains.
