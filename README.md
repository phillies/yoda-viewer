# YoDa Viewer
YOLO Dataset viewer — review and edit Ultralytics YOLO segmentation & bounding-box labels in your browser.

## Installation
```bash
uv pip install .
```

## Run
```bash
yoda
```
The server starts on port **8080** by default. If that port is in use it automatically increments until a free port is found.

### Configuration
All settings are read from environment variables (or a `.env` file):

| Variable | Description | Default |
|---|---|---|
| `YODA_IMAGE_BASE_PATH` | Root folder that contains the images | (required) |
| `YODA_LABEL_BASE_PATH` | Root folder that contains the YOLO `.txt` label files (must mirror the image folder structure) | (required) |
| `YODA_CLASS_INFO_YAML` | Path to dataset YAML with class names (Ultralytics format, `names:` key) | — |
| `YODA_COLOR_MAP_YAML` | Path to a color-map YAML (`class_id: "#RRGGBB"`) | — |
| `YODA_HOST` | Host for the uvicorn server | `0.0.0.0` |
| `YODA_PORT` | Port for the uvicorn server | `8080` |

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

## Rust Rewrite

The Rust implementation lives alongside the current Python application during migration. The Python app remains the behavioral reference until the Rust workspace reaches explicit parity checkpoints.

### Workspace

- crates/yoda-core: labels, geometry, and shared domain types
- crates/yoda-config: dataset YAML and runtime configuration
- crates/yoda-data: filesystem-backed repository layer
- crates/yoda-app: app state and action orchestration
- crates/yoda-ui: shared Dioxus components
- crates/yoda-web: Axum plus Dioxus fullstack entrypoint
- crates/yoda-desktop: Dioxus desktop entrypoint

### Rust commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
pwsh ./scripts/rust-dev.ps1 serve-desktop
pwsh ./scripts/rust-dev.ps1 serve-web
```

`dx` is required for the serve commands. If it is not installed locally yet, install the Dioxus CLI first and then rerun the script.

### Run examples (Windows and Linux)

The Rust apps read dataset settings from environment variables. The examples below set image/label folders plus class and color maps explicitly.

#### Windows (PowerShell)

Web app:

```powershell
$env:YODA_IMAGE_BASE_PATH = "example_data/images"
$env:YODA_LABEL_BASE_PATH = "labels"
$env:YODA_CLASS_INFO_YAML = "example_data/carparts-seg.yaml"
$env:YODA_COLOR_MAP_YAML = "example/color_map.yaml"
$env:YODA_HOST = "127.0.0.1"
$env:YODA_PORT = "8080"
cargo run -p yoda-web --features server
```

Desktop app:

```powershell
$env:YODA_IMAGE_BASE_PATH = "example_data/images"
$env:YODA_LABEL_BASE_PATH = "labels"
$env:YODA_CLASS_INFO_YAML = "example_data/carparts-seg.yaml"
$env:YODA_COLOR_MAP_YAML = "example/color_map.yaml"
$env:YODA_HOST = "127.0.0.1"
$env:YODA_PORT = "8080"
cargo run -p yoda-desktop
```

You can also use the helper script on Windows when `dx` is installed:

```powershell
pwsh ./scripts/rust-dev.ps1 serve-web
pwsh ./scripts/rust-dev.ps1 serve-desktop
```

#### Linux/macOS (bash)

Web app:

```bash
export YODA_IMAGE_BASE_PATH="example_data/images"
export YODA_LABEL_BASE_PATH="labels"
export YODA_CLASS_INFO_YAML="example_data/carparts-seg.yaml"
export YODA_COLOR_MAP_YAML="example/color_map.yaml"
export YODA_HOST="127.0.0.1"
export YODA_PORT="8080"
cargo run -p yoda-web --features server
```

Desktop app:

```bash
export YODA_IMAGE_BASE_PATH="example_data/images"
export YODA_LABEL_BASE_PATH="labels"
export YODA_CLASS_INFO_YAML="example_data/carparts-seg.yaml"
export YODA_COLOR_MAP_YAML="example/color_map.yaml"
export YODA_HOST="127.0.0.1"
export YODA_PORT="8080"
cargo run -p yoda-desktop
```

With bash on Linux/macOS, use `dx` directly:

```bash
dx serve --package yoda-web --platform web
dx serve --package yoda-desktop --platform desktop
```
