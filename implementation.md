# YoDa – Implementation Documentation (Versions 1–3)

## Overview

YoDa (YOLO Dataset viewer) is a browser-based tool for visualising and editing
YOLO-format segmentation and bounding-box labels on top of images. It is built
with **NiceGUI** and targets Python ≥ 3.14.

---

## Architecture

```
                  ┌──────────────┐
                  │  CLI (Typer) │   yoda.main
                  └──────┬───────┘
                         │  YoDaConfig.load(**overrides)
                  ┌──────▼───────┐
                  │    Config    │   yoda.config
                  │ (pydantic)   │   YoDaSettings + YoDaConfig
                  └──────┬───────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
   ┌──────▼──────┐ ┌────▼────┐  ┌──────▼──────┐
   │   UI layer  │ │ Dataset │  │  File ops   │
   │  (NiceGUI)  │ │ loader  │  │  tree/glob  │
   │  yoda.ui    │ │ yoda.   │  │  yoda.      │
   │             │ │ dataset  │  │  fileops    │
   └──────┬──────┘ └─────────┘  └─────────────┘
          │
   ┌──────▼──────┐
   │ Label layer │   yoda.label
   │ parse + SVG │   LabelObject, parse_yolo_labels, render_labels_to_svg
   └─────────────┘
```

---

## File Responsibilities

| File | Purpose |
|------|---------|
| `src/yoda/main.py` | Typer CLI entry-point (`yoda` command). Collects CLI options, creates `YoDaConfig`, instantiates `YoDaBrowser`, calls `ui.run()`. |
| `src/yoda/config.py` | `YoDaSettings` (pydantic-settings with `YODA_*` env prefix) and `YoDaConfig` (singleton that loads settings + colour map). |
| `src/yoda/constants.py` | Auto-generates 100 default colours via HSV rotation. |
| `src/yoda/dataset.py` | `load_class_map()` reads Ultralytics YAML `names` key → `dict[int, str]`. |
| `src/yoda/fileops.py` | `get_files()` (flat glob) and `get_file_tree()` (nested dict for NiceGUI `ui.tree`). |
| `src/yoda/label.py` | `LabelObject` dataclass, `parse_yolo_labels()` (txt → list of objects with pixel coords), `render_labels_to_svg()` (objects → SVG overlay string), `write_yolo_labels()` (save back to YOLO txt), `create_label_from_pixels()` (pixel polygon → new `LabelObject`), `delete_label()` (remove by index). |
| `src/yoda/ui.py` | `YoDaBrowser` – builds the NiceGUI page: left file-tree, centre image viewer with SVG overlay, top toolbar with display toggles and edit/draw mode buttons, right drawer with object list including per-object visibility, class editing, and delete. |

---

## Configuration

Settings are resolved in this priority (highest wins):

1. CLI flags (`--port`, `--image-base-path`, …)
2. Environment variables (`YODA_PORT`, `YODA_IMAGE_BASE_PATH`, …)
3. Defaults in `YoDaSettings`

### Colour Map

A YAML file mapping class-ID → `[R, G, B]`:

```yaml
0: [255, 0, 0]
1: [0, 255, 0]
```

If not provided, 100 auto-generated colours from `constants.py` are used.
User colours are merged on top of defaults.

### Class Info

An Ultralytics dataset YAML with a `names` key:

```yaml
names:
  0: front_bumper
  1: rear_bumper
  …
```

---

## UI Layout

```
┌────────────────────────────────────────────────────┐
│  Display: [Seg.Masks] [BBoxes] [ClassID] [Name]  │  ← toolbar
│  🔲 Toggle objects │  class legend                │
├────────┬───────────────────────────────┬───────────┤
│ Images │                               │ Objects   │
│  📁test│        (image + SVG overlay)  │ #1 wheel  │
│  📁train│                              │ #2 bumper │
│  📁val │                               │ …        │
├────────┴───────────────────────────────┴───────────┤
```

- **Left pane** – `ui.tree` from `get_file_tree()`. Click a file → loads image.
- **Centre** – `ui.interactive_image` with an SVG `content` overlay.
- **Right drawer** – opens via button; lists every label object with coloured dot, index, class name, and type (poly/bbox).
- **Toolbar** – 4 `ui.switch` toggles that re-render the SVG without reloading the image.

---

## Label Format

Each line in a `.txt` label file:

```
class_id x1 y1  x2 y2  …  xN yN
```

- Coordinates are **normalised** (0–1).
- ≥ 3 coordinate pairs → segmentation polygon; exactly 2 pairs → bounding box.
- `parse_yolo_labels()` converts normalised coords to pixel coords and computes a tight bounding box for polygons.

---

## Running

```bash
# Install (editable)
uv sync

# Start with defaults (example_data)
uv run yoda

# Custom paths
uv run yoda --image-base-path ./my_images --label-base-path ./my_labels \
             --class-info ./classes.yaml --color-map ./colors.yaml --port 9000
```

---

## Testing

```bash
# Unit tests only
uv run pytest tests/test_config.py tests/test_label.py tests/test_fileops.py -v

# E2E browser tests (requires Playwright Chromium)
uv run playwright install chromium
uv run pytest tests/test_ui_e2e.py -v

# Full suite
uv run pytest tests/ -v
```

### Test Coverage

| Suite | Tests | What it covers |
|-------|------:|----------------|
| `test_config.py` | 11 | Settings defaults, env prefix, config loading, colour map merging, fallbacks |
| `test_label.py` | 32 | Parsing segmentation/bbox labels, pixel conversion, SVG rendering with all toggle combinations, write round-trips, `create_label_from_pixels` (triangle, normalisation, bounding box, edge cases), `delete_label` (middle/first/last/invalid/single/empty/roundtrip) |
| `test_fileops.py` | 8 | File globbing, tree generation, hidden-file filtering, directory ordering |
| `test_main.py` | 3 | Port finder (free port, busy port skip, exhaustion error) |
| `test_ui_e2e.py` | 20 | Page load, folder tree, toggle switches, class legend, image viewer, object drawer, visibility buttons, class dropdowns, edit/draw mode buttons, class selector, draw mode activation, edit mode return, delete buttons present, delete reduces object count |
| **Total** | **74** | |

---

## Key Design Decisions

1. **SVG overlay instead of canvas** – NiceGUI's `interactive_image.content` accepts raw SVG; re-rendering is fast and avoids a JS round-trip.
2. **Toggles re-render, don't reload** – Parsed `LabelObject` list is cached; only the SVG string is rebuilt when a switch flips.
3. **Subprocess E2E fixture** – NiceGUI detects pytest via `PYTEST_CURRENT_TEST` env var and enters screen-test mode. The E2E fixture strips that variable so the subprocess starts a normal server.
4. **pydantic-settings with `cli_ignore_unknown_args`** – Prevents conflicts when pytest adds its own `sys.argv` entries.

---

## Version 3 – Draw & Delete

### Interaction Modes

V3 adds an **edit/draw mode** toggle to `YoDaBrowser`:

| Mode | Icon | Behaviour |
|------|------|-----------|
| **Edit** (default) | `pan_tool` | Zoom/pan via mouse, click objects in drawer for class/visibility editing |
| **Draw** | `add` | Click on image to place polygon vertices; finish with Enter or click first vertex; cancel with Escape |

The mode state is stored as `self.interaction_mode` (`"edit"` or `"draw"`).

### Toolbar Changes

```
┌──────────────────────────────────────────────────────────────────────────┐
│ ✋ Edit  ➕ Draw  [Class ▼]  │  [Seg.Masks] [BBoxes] [ClassID] [Name]  │
│                               │  🔲 Toggle objects │  class legend     │
└──────────────────────────────────────────────────────────────────────────┘
```

- **Edit button** (pan_tool) – returns to edit mode; highlighted when active.
- **Draw button** (add) – enters draw mode; highlighted when active.
- **Class selector** – dropdown to choose the class ID for newly drawn objects.

### Drawing Workflow

1. User clicks **Add** button → draw mode activates.
2. User clicks on the image → vertex added to `self.drawing_vertices`.
3. A green SVG preview (circles at vertices + connecting lines) is appended to the overlay.
4. **Enter** key or **clicking near the first vertex** → `_finish_drawing()`:
   - Calls `create_label_from_pixels()` to build a `LabelObject`.
   - Appends to `self.current_labels`, saves file, refreshes overlay.
5. **Escape** key → cancels drawing, clears vertices.

### Delete Workflow

Each object row in the right drawer now includes a **delete** icon button (`delete` Material icon). Clicking it:

1. Calls `delete_label(self.current_labels, index)` which returns a new list without the target object, with indices re-sequenced.
2. Saves the updated labels to disk.
3. Refreshes the SVG overlay.
4. Sends a `ui.notify("Object deleted")` notification *before* rebuilding the object list (important: the rebuild clears the container which deletes the triggering button, invalidating the NiceGUI slot context).

### New `label.py` Functions

```python
def create_label_from_pixels(
    pixel_points: list[tuple[float, float]],
    image_width: int,
    image_height: int,
    class_id: int = 0,
    index: int = 0,
) -> LabelObject:
    """Convert pixel-coordinate polygon vertices into a LabelObject."""

def delete_label(
    labels: list[LabelObject],
    label_index: int,
) -> list[LabelObject]:
    """Remove a label by index and re-sequence remaining indices."""
```

### Keyboard Handler

A `ui.keyboard` element captures key events in draw mode:

- **Enter** → finish polygon, create label.
- **Escape** → cancel drawing, return to edit mode.

### E2E Test Data Protection

V3 E2E tests use an `autouse` fixture that backs up all test label files before each test and restores them afterwards, preventing test side-effects from corrupting example data.
