# YoDa Rust Codebase — Context Summary

> Updated: 2026-06-30
> Branch: `feat/class-filter` (commit `8dc3f0c`)

---

## Project Overview

**YoDa** is a Rust rewrite of a Python YOLO dataset viewer/editor.
It lets you browse, view, and edit YOLO segmentation/bbox label files alongside their images.

- **Framework**: Dioxus 0.7.x (fullstack) + Axum HTTP server
- **Build**: Cargo workspace (`Cargo.toml` at root)
- **Run**: `cargo run -p yoda-web --features server`
- **Web bundle**: Not yet built; browser receives server-rendered fallback HTML at `/`

---

## Crate Layout

| Crate | Purpose |
|---|---|
| `yoda-config` | Settings (env vars `YODA_*`), class map / color map YAML loading |
| `yoda-core` | `LabelObject`, YOLO parse/write, SVG rendering |
| `yoda-data` | `TreeNode`, `FlatIndex`, `ClassIndex`, dataset scanning helpers |
| `yoda-app` | `AppState`, all actions, reducer (pure Rust state machine) |
| `yoda-ui` | Dioxus components: `ClassFilterBar`, `compute_filtered_rows`, tree, toolbar |
| `yoda-web` | Axum server, routes, `BackendState`, SSR fallback viewer |
| `yoda-desktop` | Desktop entry point (thin wrapper around yoda-web) |

---

## Key Data Structures

### `yoda-data`

```rust
// TreeNode — recursive dataset tree (files + folders)
pub struct TreeNode { pub id: String, pub label: String, pub children: Vec<TreeNode>, pub icon: NodeIcon }

// FlatIndex — flattened ordered list used for prev/next navigation
pub struct FlatIndex { pub nodes: Vec<FlatNode> }
pub struct FlatNode  { pub path: String, pub kind: NodeKind }  // kind: Image | Folder

// ClassIndex — maps dataset-relative image path → class IDs in its label file
pub struct ClassIndex { pub entries: HashMap<String, Vec<u32>> }
// Persisted to: <label_root>/.yoda_class_index.json
// Built by: ClassIndex::load_or_build(image_root, label_root, &flat_index)
```

`FilterMode` (Any / All) lives in `yoda-data::class_index` and is re-exported from `yoda-data`.

### `yoda-app`

```rust
pub struct AppState {
    // navigation
    pub tree: Vec<TreeNode>,
    pub flat_index: FlatIndex,
    pub current_image: Option<String>,

    // display toggles
    pub show_seg: bool, pub show_bbox: bool, pub show_class_id: bool, pub show_class_name: bool,

    // class filter (added in class-filter feature)
    pub filter_classes: BTreeSet<u32>,
    pub filter_mode: FilterMode,
    pub class_index: HashMap<String, Vec<u32>>,  // loaded from /api/class-index at startup
}
```

Actions: `Navigate`, `ToggleSeg`, `ToggleBbox`, `ToggleClassId`, `ToggleClassName`,
`ClassIndexLoaded`, `SetFilterClass { class_id, selected }`, `ClearClassFilter`, `SetFilterMode`.

---

## HTTP API (Axum, port 8080 default)

| Route | Description |
|---|---|
| `GET /` | SSR fallback viewer (HTML) |
| `GET /api/tree` | Full `Vec<TreeNode>` JSON |
| `GET /api/flat-index` | `FlatIndex` JSON |
| `GET /api/image?image_path=<rel>` | Raw image bytes |
| `GET /api/labels?image_path=<rel>` | `Vec<LabelObject>` JSON |
| `POST /api/labels` | Save labels + patch ClassIndex cache |
| `GET /api/class-index` | `{ entries: {path: [class_ids]}, all_class_ids: [u32] }` |
| `GET /api/class-map` | `HashMap<u32, String>` from YAML |

---

## Configuration (env vars)

| Variable | Default | Description |
|---|---|---|
| `YODA_IMAGE_BASE_PATH` | — | Root directory for images |
| `YODA_LABEL_BASE_PATH` | — | Root directory for `.txt` label files |
| `YODA_CLASS_INFO_YAML` | — | Ultralytics YAML with `names:` key |
| `YODA_COLOR_MAP_YAML` | — | Optional int→RGB overrides |
| `YODA_HOST` | `127.0.0.1` | Bind address |
| `YODA_PORT` | `8080` | Bind port |

Example dataset config is in `example_data/carparts-seg.yaml`.

---

## Class Filter Feature (implemented, committed)

- **Cache file**: `<label_root>/.yoda_class_index.json` (278 KB for 3850-image dataset)
- **Startup cost**: ~35 s first build, ~0.03 s cached restart
- **Filter counts (example dataset)**: class 0 → 870 images, classes 0+1 All → 294 images
- **UI**: `ClassFilterBar` component renders one chip per class; Any/All radio; clear button
- **Filtering**: pure client-side after one `/api/class-index` fetch at startup

---

## YOLO Label Format

```
<class_id> <x1> <y1> <x2> <y2> ...   # 4 coords = bbox (cx,cy,w,h norm), 6+ = polygon
```

---

## Build Notes

- `cargo run -p yoda-web --features server` — starts the axum server
- `dx build` (Dioxus CLI) needed to produce WASM bundle for full SPA mode
- SSR fallback is active when `target/debug/public/` is absent
- Dioxus router requires state type `FullstackState`; extra backend state goes in `Extension<Arc<T>>`

---

## Docs in This Repo

| File | Contents |
|---|---|
| `docs/class-filter-design.md` | Design doc for the class filter feature |
| `docs/rust-parity-checklist.md` | Feature parity checklist vs. Python version |
| `docs/codebase-context.md` | This file |
