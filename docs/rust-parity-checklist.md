# Rust Parity Checklist

This document freezes the current Python behavior that the Rust implementation must preserve unless a change is explicitly called out. It maps the existing Python modules and tests into the Rust milestone plan from implementation.rust.md.

## Dataset Assumptions

- Primary development fixture: example_data
- Default dataset YAML: example_data/carparts-seg.yaml
- Default image root: example_data/images
- Default label root: example_data/labels
- Label files mirror the image tree and use the same relative path with a .txt suffix.
- Supported image extensions: .jpg, .jpeg, .png, .bmp, .webp
- Hidden files and directories are ignored in the tree.

## Non-Negotiable Runtime Contracts

### Existing Python behavior to preserve

- Missing label files load as zero labels instead of an error.
- Empty label files load as zero labels.
- YOLO lines with 4 normalized coordinates are parsed as bounding boxes.
- YOLO lines with 6 or more normalized coordinates are parsed as polygons.
- Polygon labels with fewer than 3 valid points are ignored.
- Normalized coordinates remain the source of truth when writing labels back to disk.
- Label deletion reindexes remaining objects sequentially from 0.
- Directory tree nodes load lazily and only expand a directory when requested.
- Directories sort before files, with case-insensitive name ordering inside each group.
- Non-image files are excluded from the UI tree.
- Class names are loaded from an Ultralytics dataset YAML names mapping.
- Custom color map entries override generated defaults without removing non-overridden defaults.
- If the configured port is busy and the user did not explicitly pass a port, startup increments to the next available port.
- The UI persists the last opened image in session/user storage.
- Object visibility can be controlled per-object and per-class.
- Drawing a new polygon requires at least 3 points.
- Deleting an object clears or repairs selection so no stale index remains selected.

### New Rust behavior approved by plan

- The Rust app starts in locked read-only mode by default.
- Unlocking edits is explicit session state rather than an image-specific flag.
- Lock state persists across image changes until the user changes it.
- Edit-capable actions are guarded in both UI state and mutation execution paths.
- Viewer-first is a valid milestone before edit parity is complete.

### Current Python behavior that should not be treated as required parity

- Immediate write-on-change for every class dropdown change is current behavior, but Rust should preserve the user-visible outcome rather than the exact implementation detail.
- The monolithic NiceGUI class layout is not a parity target; only its behavior is.
- The current theme and Quasar-specific DOM structure are not parity targets.
- The current lack of an edit lock is intentionally replaced by the new locked-by-default model.

## Classification By Existing Test Coverage

| Area | Source | Behavior | Rust milestone |
|---|---|---|---|
| Label parsing | tests/test_label.py | Missing and empty files return [] | viewer-first |
| Label parsing | tests/test_label.py | Polygon and bbox parsing semantics | viewer-first |
| Label parsing | tests/test_label.py | Pixel conversion and bbox derivation | viewer-first |
| Label rendering | tests/test_label.py | SVG polygon rendering | viewer-first |
| Label rendering | tests/test_label.py | SVG bbox rendering | viewer-first |
| Label rendering | tests/test_label.py | Class id and class name text overlays | viewer-first |
| Label rendering | tests/test_label.py | Hidden objects are skipped | viewer-first |
| Label rendering | tests/test_label.py | Selected polygon styling | viewer-first |
| Label writing | tests/test_label.py | Deterministic write and reparse round-trip | edit-phase |
| Label editing | tests/test_label.py | Create polygon from pixels | edit-phase |
| Label editing | tests/test_label.py | Delete and reindex | edit-phase |
| File tree | tests/test_fileops.py | Lazy tree placeholder for directories | viewer-first |
| File tree | tests/test_fileops.py | Hidden and non-image filtering | viewer-first |
| File tree | tests/test_fileops.py | Directory-first ordering | viewer-first |
| Config | tests/test_config.py | Default paths and env overrides | viewer-first |
| Config | tests/test_config.py | Custom color map merge semantics | viewer-first |
| Config | tests/test_config.py | Class map loading from YAML names | viewer-first |
| UI shell | tests/test_ui_e2e.py | Page load and file tree visibility | viewer-first |
| UI shell | tests/test_ui_e2e.py | Display toggles visible | viewer-first |
| UI shell | tests/test_ui_e2e.py | Class legend visible | viewer-first |
| UI shell | tests/test_ui_e2e.py | Image load from tree selection | viewer-first |
| UI shell | tests/test_ui_e2e.py | Object list population | viewer-first |
| Viewer interaction | tests/test_ui_e2e.py | Selection by clicking image | viewer-first |
| Viewer interaction | tests/test_ui_e2e.py | Toolbar mode buttons visible | viewer-first |
| Edit UI | tests/test_ui_e2e.py | Object delete buttons visible | edit-phase |
| Edit UI | tests/test_ui_e2e.py | Class dropdowns visible and usable | edit-phase |
| Edit UI | tests/test_ui_e2e.py | Draw mode button flow | edit-phase |
| Edit UI | tests/test_ui_e2e.py | Delete action reduces object count | edit-phase |

## Planned Rust Milestone Boundaries

### Viewer-first

- Dataset config and fixture defaults
- Lazy tree expansion
- Image loading
- Label parsing
- SVG overlays for polygons, bboxes, and label text
- Class legend and object list display
- Selection and selection highlighting
- Zoom, fit, and reset basics
- Locked-by-default session state visible in the UI

### Edit-phase

- Explicit unlock flow
- Class reassignment
- Delete selected or targeted object
- Polygon drawing and finalize/cancel flow
- Atomic label writes
- Mutation rejection while locked

### Optional polish

- Fine-grained keyboard shortcut parity beyond core viewer and edit workflows
- Last-opened image persistence across targets if transport-specific work makes this expensive initially
- Packaging polish beyond Linux-first desktop and documented self-hosted web deployment

## Open Gaps To Watch During Port

- The Python tests do not currently cover the new lock-state contract, so Rust must add those tests.
- Python selection E2E coverage is weak on exact hit-testing assertions; Rust should strengthen unit coverage in the geometry layer.
- Python config tests do not cover malformed YAML handling in depth; Rust should add explicit failure-path tests.