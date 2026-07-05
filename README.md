# YoDa Viewer

**YO**LO **Da**taset viewer — review and edit [Ultralytics YOLO](https://docs.ultralytics.com/)
segmentation and bounding-box labels in your browser or as a desktop app.

YoDa points at an existing YOLO-format dataset on disk (images + mirrored `.txt` label files),
renders the annotations as interactive overlays, lets you filter the dataset by class, and —
after explicitly unlocking edit mode — lets you reassign classes, delete objects, and draw new
polygons, with every change written straight back to the label files.

- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Feature Documentation](#feature-documentation)
  - [UI Layout](#ui-layout)
  - [Dataset Tree](#dataset-tree)
  - [Class Filter](#class-filter)
  - [Image Viewer & Overlays](#image-viewer--overlays)
  - [Zoom & Pan](#zoom--pan)
  - [Class Legend](#class-legend)
  - [Object List](#object-list)
  - [Editing](#editing)
  - [Keyboard Shortcuts](#keyboard-shortcuts)
  - [Status Bar & Messages](#status-bar--messages)
  - [Persistence & Caching](#persistence--caching)
- [YOLO Label Format](#yolo-label-format)
- [HTTP API Reference](#http-api-reference)
- [Deployment Modes](#deployment-modes)
- [Architecture](#architecture)
- [Development](#development)
- [Further Documentation](#further-documentation)

---

## Quick Start

### Requirements

- Rust 1.85+ (edition 2024; pinned via `rust-toolchain.toml`)
- `dx` CLI (Dioxus) for the hydrated web UI and dev workflows: `cargo install dioxus-cli`

### Web App

```bash
export YODA_IMAGE_BASE_PATH="example_data/images"
export YODA_LABEL_BASE_PATH="example_data/labels"
export YODA_CLASS_INFO_YAML="example_data/carparts-seg.yaml"
export YODA_COLOR_MAP_YAML="example/color_map.yaml"
cargo run -p yoda-web --features server
```

The server starts on port **8080** by default and prints the bound address. If the port is
busy *and you did not set `YODA_PORT` explicitly*, it automatically increments until a free
port is found (up to 20 attempts).

> **Note:** for the full interactive single-page app the WASM bundle must have been built
> (`dx build`, or run via `dx serve`). Without it the server falls back to a server-rendered,
> read-only viewer — see [Deployment Modes](#deployment-modes).

### Desktop App

```bash
export YODA_IMAGE_BASE_PATH="example_data/images"
export YODA_LABEL_BASE_PATH="example_data/labels"
cargo run -p yoda-desktop
```

The desktop app starts its own local backend (binding `127.0.0.1` on the first free port at
or above `YODA_PORT`) and opens the same UI in a native window.

On Windows (PowerShell):

```powershell
$env:YODA_IMAGE_BASE_PATH = "example_data/images"
$env:YODA_LABEL_BASE_PATH = "example_data/labels"
cargo run -p yoda-web --features server   # or: cargo run -p yoda-desktop
```

### Docker

The `Dockerfile` builds the WASM bundle and the server into a single image:

```bash
docker build -t yoda-viewer .
docker run -p 8080:8080 \
  -v /path/to/images:/data/images:ro \
  -v /path/to/labels:/data/labels \
  yoda-viewer
```

Image defaults: `YODA_IMAGE_BASE_PATH=/data/images`, `YODA_LABEL_BASE_PATH=/data/labels`,
`YODA_HOST=0.0.0.0`, `YODA_PORT=8080`. Mount the label directory **writable** — YoDa writes
label edits and its class-index cache there.

> ⚠️ There is currently no authentication and no read-only switch: anyone who can reach the
> port can edit labels. Only expose the server on trusted networks.

---

## Configuration

All settings are read from environment variables (a `.env` file works too):

| Variable | Description | Default |
|---|---|---|
| `YODA_IMAGE_BASE_PATH` | Root folder containing the images | `example_data/images` |
| `YODA_LABEL_BASE_PATH` | Root folder containing YOLO `.txt` label files; must mirror the image folder structure | `example_data/labels` |
| `YODA_CLASS_INFO_YAML` (alias `YODA_CLASS_INFO`) | Dataset YAML with class names (Ultralytics format, `names:` key) | `example_data/carparts-seg.yaml` |
| `YODA_COLOR_MAP_YAML` (alias `YODA_COLOR_MAP`) | Color-map YAML overriding per-class colors | — |
| `YODA_HOST` | Bind host for the web server | `127.0.0.1` (web) |
| `YODA_PORT` | Port for the web server. When set explicitly, it is used as-is (no auto-increment) | `8080` |

Paths are canonicalized at startup; a missing image directory fails startup with a clear
error. The label directory may be created lazily on first save.

### Class names YAML (`YODA_CLASS_INFO_YAML`)

Standard Ultralytics dataset YAML. Both `names:` forms are supported:

```yaml
# mapping form                      # sequence form
names:                              names:
  0: back_bumper                      - back_bumper
  1: front_bumper                     - front_bumper
  2: wheel                            - wheel
```

Only the `names:` key is read; all other keys are ignored. If the file is missing or
malformed, YoDa runs with an empty class map and displays classes as `class <id>`.

### Color map YAML (`YODA_COLOR_MAP_YAML`)

Maps class IDs to display colors. Two value forms are supported:

```yaml
0: "#e25d3f"        # hex string
1: [127, 159, 75]   # RGB triple
```

Classes without an entry get a deterministic auto-generated color (golden-angle HSV — the
same class ID always gets the same color, and neighboring IDs get visually distinct hues).
Custom entries override individual classes without affecting the generated defaults.

---

## Feature Documentation

### UI Layout

The app is a three-panel layout:

```
┌────────────┬───────────────────────────────┬──────────────┐
│  Dataset   │  Toolbar                      │  Classes     │
│  (filter + │  ───────────────────────────  │  (legend)    │
│   tree)    │  Image viewport               │  ──────────  │
│            │  (image + label overlays)     │  Objects     │
│            │  ───────────────────────────  │  (per-object │
│            │  Status bar                   │   controls)  │
└────────────┴───────────────────────────────┴──────────────┘
```

On narrow windows (≤1180 px) the right panel moves below the content; on very narrow windows
(≤860 px) all panels stack vertically.

### Dataset Tree

- The left panel shows the image folder hierarchy under `YODA_IMAGE_BASE_PATH`.
- The full tree structure is indexed once at server startup and loaded by the client in one
  request, so expanding folders is instant (no per-folder round trips).
- **Sorting:** folders before files, then case-insensitive alphabetical within each group.
- **Filtering:** hidden entries (names starting with `.`) are excluded; only supported image
  files are shown: `.jpg`, `.jpeg`, `.png`, `.bmp`, `.webp`. Non-image files (including the
  label `.txt` files) never appear.
- Click a folder row to expand/collapse it (`▸` / `▾` arrow); click an image row to open the
  image. The currently open image is highlighted.
- A folder count and image count are logged at startup; the tree shows a loading message
  while the initial index is fetched.

### Class Filter

The filter bar sits above the tree (it appears only when a class map is configured):

- **Chips** — one per class from the dataset YAML, each with its class color. Click a chip
  to toggle it. Active chips are outlined in the class color.
- **Any / All mode** — with multiple chips selected, *Any* shows images containing at least
  one selected class; *All* shows only images containing every selected class.
- **Filtered tree** — while a filter is active, the tree shows only matching images and the
  folders that (transitively) contain them; empty folders are hidden. Expand/collapse still
  works within the filtered view.
- **Clear** — the `× Clear` button in the filter bar, or the `×` on the filter badge in the
  status bar, removes all filter chips at once.
- The filter is powered by a **persistent class index** (see
  [Persistence & Caching](#persistence--caching)) that knows which class IDs occur in every
  image's label file.

### Image Viewer & Overlays

Selecting an image loads it together with its labels and renders SVG overlays on top:

- **Segmentation masks** — polygons drawn with a semi-transparent fill (30% opacity) and a
  solid outline in the class color.
- **Bounding boxes** —
  - real bbox labels: rectangle with 20% fill in the class color;
  - polygon labels: their *derived* bounding box drawn as a dashed, unfilled rectangle
    (useful to compare mask extents against detector-style boxes).
- **Text labels** — optional class ID and/or class name per object, rendered on a dark chip
  anchored to the object's top-left corner. Text size scales proportionally with image width
  (baseline 640 px) so labels stay readable on large images and never shrink below the
  baseline size on small ones.
- **Selected object** — the selected polygon is highlighted with a white dashed outline and
  increased fill opacity.

Toolbar display toggles (all act instantly, nothing is persisted):

| Button | Effect | Default |
|---|---|---|
| **Mask** | show/hide segmentation polygons | on |
| **BBox** | show/hide bounding boxes (real + derived) | off |
| **Class ID** | show numeric class ID chip per object | off |
| **Class Name** | show class name chip per object | off |

### Zoom & Pan

- **Mouse wheel** — zoom in/out, centered on the cursor position. Zoom range: 0.25× – 6×.
- **Click & drag** — pan the image (the cursor switches to a grab hand).
- **Double-click** — reset zoom and pan to the default view. *Only while editing is locked*;
  in unlocked mode double-click is reserved for selecting objects on the canvas.
- Zoom and pan reset automatically when a different image is opened.
- While polygon draw mode is active, dragging is disabled so clicks place vertices instead.

### Class Legend

The *Classes* section in the right panel lists every class from the dataset YAML **plus** any
class IDs that occur in the current image but are missing from the YAML (shown as
`class <id>`). Each row shows:

- the class color swatch,
- the class name,
- a **Hide / Show** button that toggles visibility of *all* objects of that class at once.

Class-level hiding combines with per-object hiding: an object is visible only if neither its
class nor the object itself is hidden. Visibility state is per-session and resets on reload.

### Object List

The *Objects* section lists every annotation in the current image:

- **Color swatch** — the object's class color.
- **Visible / Hidden button** — toggles that single object's visibility. The button also
  reflects class-level hiding (it reads *Hidden* when the object's class is hidden).
- **Name** — `#<n> <class name> (<polygon|bbox>)`, numbering from 1 in file order.
- **Class dropdown** — reassigns the object to another class. The list contains all classes
  from the YAML plus any classes present in the image. Disabled while locked. Changing the
  class **saves the label file immediately**.
- **Delete button** — removes the annotation. Disabled while locked. Saves immediately.
- **Selection** — clicking a row selects/deselects the object (highlighted in the list and on
  the canvas). In unlocked mode, double-clicking a shape on the canvas does the same.
  Deleting an object clears its selection; remaining objects are renumbered sequentially.

### Editing

YoDa starts in **locked** (read-only) mode. All mutating actions are guarded twice — in the
UI (disabled controls) and in the state layer (blocked actions show an explanatory message).

- **Unlock Editing / Lock Editing** — toolbar toggle; the adjacent status pill shows
  *Locked* (amber) or *Unlocked* (green). Lock state persists while switching images and
  resets to locked on reload.
- **Change an object's class** — pick a new class in the object row's dropdown. Saved to
  disk immediately; the status message confirms *Labels saved*.
- **Delete an object** — the row's *Delete* button, or select the object and press
  `Delete`/`Backspace`. Saved immediately.
- **Draw a new polygon** (unlocked only):
  1. Click **Draw Polygon** in the toolbar (button highlights; a vertex counter appears).
  2. Choose the class for the new object in the adjacent dropdown (defaults to the lowest
     class ID from the YAML).
  3. Click on the image to place vertices. A dashed preview polygon and a "rubber band" line
     to the cursor show the shape in progress.
  4. Close the polygon by clicking the highlighted ring around the **first vertex** (it
     lights up when the cursor is near), or press **Enter** / click **Finish**. Requires at
     least 3 vertices.
  5. **Escape** / **Cancel** discards the in-progress polygon.

  The new object is appended, saved to disk immediately, and the app returns to edit mode.
- **Not yet supported:** drawing bounding boxes, moving/editing existing vertices, undo.
  See `docs/implementation/` for the designs of these planned features.

### Keyboard Shortcuts

Shortcuts are global (ignored while typing in an input, dropdown, or text area):

| Key | Action |
|---|---|
| `Delete` / `Backspace` | Delete the selected object (unlocked mode) |
| `Enter` | Finish the polygon being drawn |
| `Escape` | Cancel the polygon being drawn |

### Status Bar & Messages

The bar under the viewport shows: current image name, pixel dimensions, object count, mode
(*Viewer* when locked, *Edit* when unlocked), and — when a class filter is active — a filter
badge listing the selected class names with an inline `×` to clear.

Above the viewport, transient messages appear for state changes (*Image loaded*,
*Labels saved*) and errors (failed loads/saves, blocked edit attempts such as
*"Unlock editing to delete objects."*).

### Persistence & Caching

- **Label saves are immediate.** Every edit (class change, delete, new polygon) rewrites the
  image's label file on the spot. There is no separate save button, no undo, and no
  dirty-state; the file on disk is the source of truth.
- **Class index cache.** To power the class filter, YoDa scans every label file once and
  stores the result in `<YODA_LABEL_BASE_PATH>/.yoda_class_index.json`. On later startups the
  cache is loaded, new images are scanned incrementally, and entries for deleted images are
  pruned. Saves made through YoDa update the cache; **edits made by external tools while the
  server runs are not detected** — restart the server (or delete the cache file) to pick
  them up.
- **Session state is ephemeral.** Display toggles, visibility, selection, zoom/pan, filter
  selection, and lock state all reset on page reload.

---

## YOLO Label Format

YoDa reads and writes standard Ultralytics YOLO annotation files: one `.txt` per image, at
the same relative path under the label root as the image under the image root, with the
extension replaced (`train/img1.jpg` ↔ `train/img1.txt`).

Each non-empty line is one object; all coordinates are **normalized to [0, 1]**:

```
<class_id> <cx> <cy> <w> <h>                    # exactly 4 values  → bounding box
<class_id> <x1> <y1> <x2> <y2> <x3> <y3> ...    # ≥6 values, even   → polygon
```

Parsing rules (matching the Ultralytics tooling behavior):

- A missing or empty label file means *no labels* — it is not an error.
- Bounding boxes are center-x, center-y, width, height.
- Polygons need at least 3 points (6 coordinates); odd coordinate counts are invalid.
- Class IDs are non-negative integers; they do not need to appear in the dataset YAML.
- **A file containing any malformed line currently loads as empty** (no partial results) —
  see `docs/implementation/correctness/01-partial-label-parse.md` for the planned fix.

Writing rules:

- Normalized coordinates are the source of truth and are written back with 6 decimal places.
- Object order is preserved; deleting an object reindexes the rest sequentially.
- Parent directories of the label file are created as needed.

---

## HTTP API Reference

All endpoints live under `/api`. Responses are JSON unless noted. Errors return an
appropriate status code with body `{"code": "<machine_code>", "message": "<detail>"}`.

Image paths in query parameters are dataset-relative (forward slashes) or absolute; every
path is canonicalized and **must resolve inside the image root**, otherwise `403 forbidden`.

| Method & Path | Description |
|---|---|
| `GET /api/health` | Liveness + version: `{"status":"ok","version":"0.2.0"}` |
| `GET /api/tree/flat` | Full dataset tree as a flat node list: `{nodes: [{id, parent_id, name, kind: "Folder"\|"Image", path}], image_count}`. Node IDs equal their index; order is depth-first, folders first. |
| `GET /api/tree/status` | `{node_count, image_count}` |
| `GET /api/tree` | *(legacy)* Root-level nodes in recursive `TreeNode` form with lazy placeholders |
| `GET /api/tree/children?path=…` | *(legacy)* Children of one directory |
| `GET /api/image?image_path=…` | Raw image bytes with correct `Content-Type` (jpeg/png/bmp/webp) |
| `GET /api/image/metadata?image_path=…` | `{image_path, width, height}` |
| `GET /api/labels?image_path=…` | `{image_path, label_path, width, height, labels: [LabelObject]}` |
| `PUT /api/labels?image_path=…` | Body `{labels: [LabelObject]}` — writes the label file, updates the class-index cache, returns the freshly re-parsed labels (same shape as GET) |
| `GET /api/class-map` | `{class_map: {"<id>": "<name>"}}` from the dataset YAML |
| `GET /api/color-map` | `{color_map: {"<id>": [r, g, b]}}` — defaults merged with the color-map YAML |
| `GET /api/class-index` | `{entries: {"<relative path>": [class ids]}, all_class_ids: […]}` |

`LabelObject` shape (as serialized):

```json
{
  "index": 0,
  "class_id": 2,
  "label_type": "Polygon",            // or "Bbox"
  "normalized_coords": [0.1, 0.2, …], // source of truth
  "pixel_points": [{"x": 64.0, "y": 96.0}, …],
  "pixel_bbox": {"x": 64.0, "y": 96.0, "width": 128.0, "height": 288.0},
  "visible": true
}
```

---

## Deployment Modes

YoDa has one backend and three front doors:

1. **Hydrated web app (primary).** When Dioxus client assets (`public/` with the WASM
   bundle) exist next to the server binary, the full single-page app is served. Get there
   with `dx serve --package yoda-web --platform web` during development, `dx build` +
   `cargo run` for a local production run, or the Docker image (which builds the bundle in).
2. **SSR fallback viewer.** When no client assets are found, `cargo run -p yoda-web` serves
   a server-rendered, **read-only** page instead (a startup log warns about this). It shows
   the dataset tree, the selected image (`/?image_path=…`) with mask + name overlays, the
   class legend, and the object list, and supports mouse zoom/pan — but no display toggles,
   no filtering, and no editing. The full JSON API is available in this mode, so it is
   usable as a headless API server.
3. **Desktop app.** `yoda-desktop` embeds the same UI in a native window and spawns the
   backend on `127.0.0.1` automatically. Functionally identical to the hydrated web app.

---

## Architecture

Cargo workspace with one-directional layering:

| Crate | Responsibility |
|---|---|
| `crates/yoda-core` | Domain types (`LabelObject`, `Point`, `PixelBBox`), YOLO parsing/serialization, SVG overlay rendering, geometry (hit-testing, colors) |
| `crates/yoda-config` | Environment/YAML configuration (`YoDaSettings`, class map, color map) |
| `crates/yoda-data` | Filesystem repository (`DatasetRepository` trait + local impl), dataset tree scanning (`FlatIndex`), class index with disk cache |
| `crates/yoda-app` | UI-agnostic application state: `AppState`, `AppAction` reducer (`apply_action`), effects (`PersistLabels`), lock/mode rules |
| `crates/yoda-ui` | Shared Dioxus components (tree, toolbar, viewer overlays, panels), pan/zoom + shortcut scripts, API client |
| `crates/yoda-web` | Axum server: JSON API, Dioxus fullstack integration, SSR fallback |
| `crates/yoda-desktop` | Dioxus desktop shell that hosts the backend locally |

State management follows a reducer pattern: components dispatch `AppAction`s,
`apply_action` mutates `AppState` and returns effects (e.g. *persist labels*), which the UI
layer executes against the API. Edit-guarding (lock mode) is enforced inside the reducer, so
every frontend gets it for free.

## Development

```bash
cargo fmt --all                                                        # format
cargo clippy --workspace --all-targets --all-features -- -D warnings   # lint
cargo test --workspace                                                 # test

# hot-reloading dev servers (requires dx CLI)
dx serve --package yoda-web --platform web
dx serve --package yoda-desktop --platform desktop
# convenience wrapper (Windows PowerShell):
pwsh ./scripts/rust-dev.ps1 serve-web        # or serve-desktop
```

Lints are strict: `unsafe_code` is forbidden; `unwrap()`, `todo!()`, and `dbg!()` are denied
workspace-wide. Tests include unit tests per crate, Axum handler tests, and an insta snapshot
of the SVG overlay renderer.

## Further Documentation

- [`docs/use-cases/`](docs/use-cases/README.md) — task-oriented walkthroughs of what you can
  do with the current version.
- [`docs/optimizations-and-features.md`](docs/optimizations-and-features.md) — codebase
  analysis: known issues, optimization opportunities, and future feature ideas.
- [`docs/next-features.md`](docs/next-features.md) — earlier issue/feature inventory.
- [`docs/implementation/`](docs/implementation/README.md) — one design document per planned
  fix/feature, with effort estimates and a dependency graph.
- [`docs/codebase-context.md`](docs/codebase-context.md),
  [`docs/rust-parity-checklist.md`](docs/rust-parity-checklist.md) — background on the
  Python→Rust rewrite and preserved behaviors.
