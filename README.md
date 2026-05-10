# YoDa Viewer
YOLO Dataset viewer — review and edit Ultralytics YOLO segmentation & bounding-box labels in your browser.

## Build & Run

### Requirements
- Rust 1.80+
- `dx` CLI (Dioxus): `cargo install dioxus-cli`

### Web App
```bash
cargo run -p yoda-web --features server
```
The server starts on port **8080** by default. If that port is in use it automatically increments until a free port is found.

### Desktop App
```bash
cargo run -p yoda-desktop
```

### Configuration
All settings are read from environment variables (or a `.env` file):

| Variable | Description | Default |
|---|---|---|
| `YODA_IMAGE_BASE_PATH` | Root folder that contains the images | (required) |
| `YODA_LABEL_BASE_PATH` | Root folder that contains the YOLO `.txt` label files (must mirror the image folder structure) | (required) |
| `YODA_CLASS_INFO_YAML` | Path to dataset YAML with class names (Ultralytics format, `names:` key) | — |
| `YODA_COLOR_MAP_YAML` | Path to a color-map YAML (`class_id: "#RRGGBB"`) | — |
| `YODA_HOST` | Host for the web server | `127.0.0.1` |
| `YODA_PORT` | Port for the web server | `8080` |

## Features

### Viewing
- **File tree** on the left shows the image folder hierarchy; click any image to open it.
- **Segmentation masks** and/or **bounding boxes** are drawn as SVG overlays on top of the image.
- Toggle overlays with the **Show Bounding Boxes** / **Show Segmentation** checkboxes.
- Toggle **Class ID** and **Class Name** display on each object.
- **Zoom & pan**: mouse-wheel zoom, click-and-drag pan, plus toolbar buttons for *Fit to screen*, *Zoom in*, *Zoom out*, and *100 %*.

### Right drawer — Class legend
- Every class present in the current image is listed with its colour and name.
- Each class has a **checkbox** — uncheck it to hide *all* objects of that class at once.

### Right drawer — Object list
- Each detected object is shown with:
  - An **eye icon** (👁) to toggle visibility of that single object.
  - A **class dropdown** to reassign the object to a different class. Changes are **saved to disk** immediately.
  - A **type badge** (`[poly]` for segmentation, `[bbox]` for bounding box).

### Editing
- **Change class**: select a new class from the dropdown next to any object — the label file is updated on disk automatically.
- **Hide / show by class**: use the class-legend checkboxes.
- **Hide / show individual objects**: use the eye-icon buttons in the object list.

## Workspace

- **crates/yoda-core**: labels, geometry, and shared domain types
- **crates/yoda-config**: dataset YAML and runtime configuration
- **crates/yoda-data**: filesystem-backed repository layer and dataset tree indexing
- **crates/yoda-app**: app state and action orchestration
- **crates/yoda-ui**: shared Dioxus components
- **crates/yoda-web**: Axum plus Dioxus fullstack entrypoint
- **crates/yoda-desktop**: Dioxus desktop entrypoint

## Development

```bash
# Format
cargo fmt --all

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Test
cargo test --workspace

# Serve web app (requires dx CLI)
pwsh ./scripts/rust-dev.ps1 serve-web
dx serve --package yoda-web --platform web

# Serve desktop app (requires dx CLI)
pwsh ./scripts/rust-dev.ps1 serve-desktop
dx serve --package yoda-desktop --platform desktop
```

Install the Dioxus CLI if not already present:
```bash
cargo install dioxus-cli
```

## Example Usage

Set the required dataset environment variables and run:

**Web app:**
```bash
export YODA_IMAGE_BASE_PATH="example_data/images"
export YODA_LABEL_BASE_PATH="labels"
export YODA_CLASS_INFO_YAML="example_data/carparts-seg.yaml"
export YODA_COLOR_MAP_YAML="example/color_map.yaml"
cargo run -p yoda-web --features server
```

**Desktop app:**
```bash
export YODA_IMAGE_BASE_PATH="example_data/images"
export YODA_LABEL_BASE_PATH="labels"
export YODA_CLASS_INFO_YAML="example_data/carparts-seg.yaml"
export YODA_COLOR_MAP_YAML="example/color_map.yaml"
cargo run -p yoda-desktop
```

On Windows (PowerShell):
```powershell
$env:YODA_IMAGE_BASE_PATH = "example_data/images"
$env:YODA_LABEL_BASE_PATH = "labels"
$env:YODA_CLASS_INFO_YAML = "example_data/carparts-seg.yaml"
$env:YODA_COLOR_MAP_YAML = "example/color_map.yaml"
cargo run -p yoda-web --features server
# or
cargo run -p yoda-desktop
```
