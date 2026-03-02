## Plan: YoDa Viewer — Version 1 Implementation

**TL;DR**: Complete the V1 browser-based YOLO segmentation label viewer by fixing existing bugs (CLI args ignored, color map disconnect, env prefix, type mismatches), then implementing the missing V1 features: separate bounding box / segmentation mask / class ID / class name toggle controls, a right-side object list showing each object's index + class name, and wiring the user-provided color map through the full rendering pipeline. The layout keeps the left tree panel + center image, adds a top toolbar for display toggles, and a right drawer for the per-image object list. Architecture will be designed with V2/V3 extensibility in mind (per-object visibility, class editing, drawing). Tests cover unit tests for parsing/config/fileops and Playwright E2E for the browser UI.

**Steps**

### Phase 0 — Project Setup & Bug Fixes

1. **Add `[build-system]` table to pyproject.toml** — add `hatchling` or `setuptools` build-system section so the project can be properly installed with `uv`.

2. **Add test dependencies to pyproject.toml** — add `pytest`, `playwright`, and `pytest-playwright` to the dev dependency group.

3. **Fix env prefix in src/yoda/config.py** — change `env_prefix="my_prefix_"` to `env_prefix="YODA_"` in `YoDaSettings.model_config`.

4. **Wire CLI args to config in src/yoda/main.py** — pass the Typer-parsed arguments (`image_base_path`, `label_base_path`, `class_info`, `color_map`, `host`, `port`) into `YoDaConfig.load()` so they override defaults. Add an `overrides` parameter to `YoDaConfig.load()` or construct `YoDaSettings` with explicit values when CLI args are provided.

5. **Fix color map loading in src/yoda/config.py** — `load_color_map()` returns `dict[int, list]` (from YAML `[r,g,b]` arrays) but `get_color_string()` expects tuples. Normalize the loaded YAML to `dict[int, tuple[int,int,int]]`. Merge with the default `COLOR_MAP` from constants so missing class IDs fall back to auto-generated colors.

6. **Fix color map path default in src/yoda/config.py** — the default points to `example_data/color_map.yaml` but the file is at `example/color_map.yaml`. Change to `None` (no default color map) since a color map is optional.

7. **Unify color map usage** — make src/yoda/label.py and src/yoda/ui.py consume colors from an injected/configured color map (through `YoDaConfig` or passed as parameter) rather than importing directly from `constants.py`. The constants module remains as the fallback default.

### Phase 1 — Data Model Refactoring

8. **Create a `LabelObject` dataclass in src/yoda/label.py** — represent a single parsed object with fields: `index: int`, `class_id: int`, `label_type: Literal["bbox", "polygon"]`, `normalized_coords: list[float]`, `pixel_points: list[tuple[float, float]]` (for polygon) or `pixel_bbox: tuple[float, float, float, float]` (for bbox: x, y, w, h). This structured representation enables V2/V3 features (editing, filtering, drawing) and separates parsing from SVG rendering.

9. **Refactor `parse_yolo()` in src/yoda/label.py** — split into two functions:
   - `parse_yolo_labels(file_path, image_width, image_height) -> list[LabelObject]` — pure parsing, returns structured data.
   - `render_labels_to_svg(labels, color_map, show_bbox, show_segmask, show_class_id, show_class_name, class_map) -> str` — takes `list[LabelObject]` and toggle flags, produces SVG string. This enables the UI to re-render without re-parsing.

10. **Update src/yoda/dataset.py** — remove the duplicate `YoDa` class. The `YoDaBrowser` will own data loading. If `dataset.py` is still needed, it should only contain helper functions for resolving dataset YAML paths (for V2+). For now, the `load_dataset()` stub can remain but mark it explicitly as not-yet-implemented.

### Phase 2 — UI Layout & Controls

11. **Restructure src/yoda/ui.py layout** — implement the layout:
    - **Left**: `ui.splitter` before-pane with file tree (existing, keep as-is).
    - **Center** (splitter after-pane): top toolbar row + image viewer area.
    - **Right drawer**: `ui.right_drawer` containing the per-image object list. Toggled open/closed via a button in the toolbar.

12. **Add display toggle controls to the toolbar in src/yoda/ui.py** — replace the single "Show Overlay" switch with four independent switches:
    - `Show Bounding Boxes` (default: off)
    - `Show Segmentation Masks` (default: on)
    - `Show Class ID` (default: off)
    - `Show Class Name` (default: off)

    Each toggle calls a `refresh_overlay()` method that re-renders SVG from the cached `list[LabelObject]` using `render_labels_to_svg()` with current toggle states.

13. **Implement the object list panel in the right drawer** — when an image is loaded, populate the drawer with a list of `LabelObject` entries. Each row shows:
    - A colored circle/dot (matching the object's class color)
    - Object index (1-based): e.g. `#1`
    - Class name (from `class_map`) or class ID if no name available

    Use a `ui.column` with `ui.row` per object. This list structure is designed to be extended in V2 with per-object visibility toggles and class selectors.

14. **Implement SVG text rendering for class ID / class name** — in `render_labels_to_svg()`, when `show_class_id` or `show_class_name` is `True`, add `<text>` SVG elements positioned at the top-left of each object's bounding box (for polygons, compute the min-x/min-y from points). Use contrasting colors (white text with dark outline or a small background rect) for readability.

15. **Implement bounding box rendering for segmentation objects** — when `show_bbox` is `True`, compute the axis-aligned bounding box from pixel polygon points and render `<rect>` with dashed stroke (to differentiate from filled segmask). For objects that are already bounding boxes (4-coord format), render the same rect but solid stroke.

### Phase 3 — NiceGUI Integration Fixes

16. **Fix `ui.run()` usage in src/yoda/main.py** — NiceGUI's `ui.run()` does not accept a callable. Instead, use `@ui.page('/')` decorator to register the page, then call `ui.run()` without arguments. Refactor `YoDaBrowser.render()` to be called from within the page function.

17. **Handle missing label files gracefully** — if a label file doesn't exist for an image, show the image without overlay and display "No labels found" in the object list. Currently `parse_yolo()` returns `""` but the UI doesn't communicate this to the user.

18. **Keep the class legend in the toolbar** — retain the existing color-coded class legend showing all dataset classes (from the YAML). This is separate from the per-image object list in the right drawer.

### Phase 4 — Testing

19. **Create test directory structure** — add `tests/` at project root with:
    - `tests/conftest.py` — shared fixtures (temp directories with test images/labels, config fixtures)
    - `tests/test_label.py` — unit tests for `parse_yolo_labels()` and `render_labels_to_svg()`
    - `tests/test_fileops.py` — unit tests for `get_files()` and `get_file_tree()`
    - `tests/test_config.py` — unit tests for `YoDaConfig`, color map loading, settings resolution
    - `tests/test_ui_e2e.py` — Playwright E2E tests

20. **Unit tests for label parsing** — test that `parse_yolo_labels()` correctly parses:
    - Segmentation polygons (multi-vertex)
    - Bounding boxes (4-value format)
    - Empty files / missing files
    - Multi-object files
    Verify `LabelObject` field values against known example data labels.

21. **Unit tests for SVG rendering** — test that `render_labels_to_svg()`:
    - Respects toggle flags (bbox on/off, segmask on/off, labels on/off)
    - Uses correct colors from the provided color map
    - Produces valid SVG elements

22. **Unit tests for config** — test env variable loading (with `YODA_` prefix), color map YAML parsing, fallback to default colors, CLI argument override flow.

23. **Unit tests for fileops** — test tree generation with nested dirs, image filtering, hidden file skipping.

24. **Playwright E2E tests** — using `pytest-playwright`:
    - Start the NiceGUI app with example data
    - Verify file tree renders with expected folders/images
    - Click an image file → verify `InteractiveImage` becomes visible
    - Toggle bounding box / segmentation switches → verify SVG content changes
    - Verify object list in right drawer shows correct count and class names
    - Test with missing label file → verify graceful handling

### Phase 5 — Documentation

25. **Create `implementation.md`** — document the full V1 implementation plan (this document), architecture decisions, file responsibilities, and how to run/test.

---

## Verification

- `uv run pytest tests/test_label.py tests/test_fileops.py tests/test_config.py` — all unit tests pass
- `uv run pytest tests/test_ui_e2e.py` — all Playwright E2E tests pass
- `uv run yoda` — starts on `localhost:8080`, file tree loads example_data images, clicking an image shows it with segmentation masks, bounding box / class ID / class name toggles work, right drawer shows indexed object list
- Manual verification: load `example_data/carparts-seg.yaml` class names, toggle all four display modes, verify colors match when a custom color map is provided

---

## Key Decisions

- **Object list format**: index + class name (e.g. `#1 front_bumper`)
- **Layout**: top toolbar for toggles + right drawer for object list, not a 3-pane splitter
- **Color map strategy**: merge user YAML over default auto-generated colors; inject via config rather than importing constants directly
- **Data model**: introduce `LabelObject` dataclass to separate parsing from rendering — enables V2 editing/filtering and V3 drawing without re-architecting
- **`dataset.py`**: minimized for V1; kept as a placeholder for future `load_dataset()` implementation
- **NiceGUI page pattern**: use `@ui.page('/')` decorator instead of passing callable to `ui.run()`

---

## Architecture (V2/V3 Extensibility Notes)

The `LabelObject` dataclass and separated parse/render pipeline are intentionally designed so that:

- **V2** can add a `visible: bool` field to `LabelObject` and a class-level filter — `render_labels_to_svg()` already accepts the label list and can skip hidden objects. The right-drawer object list rows can gain toggle switches without layout changes.
- **V2** class editing can mutate `LabelObject.class_id` and call `refresh_overlay()` to re-render.
- **V3** drawing new objects means appending a new `LabelObject` to the list. The SVG rendering and object list update are already decoupled from file I/O.
- **V3** deletion means removing a `LabelObject` from the list and re-rendering.
