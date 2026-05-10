## Rust Implementation Plan

This document turns the approved Dioxus rewrite direction into an implementation sequence that can be reviewed before code starts. The goal is to keep every segment small enough to build, test, and review independently while preserving a clean path to full parity with the current Python application.

## Scope

- Primary targets: web and desktop
- Primary desktop platform: Linux
- Secondary desktop platform: Windows, after Linux is stable
- Initial delivery strategy: viewer-first
- Follow-up delivery strategy: editing parity after viewer-first is stable
- Initial access mode: read-only at startup, with explicit user unlock for edits
- Sample dataset and fixture source: `example_data`

## Operating Assumptions

- The current Python implementation remains the behavioral reference until the Rust app passes explicit parity gates.
- The web target is not browser-only WASM. It is a self-hosted fullstack app with a Rust backend running near the dataset.
- The first Rust release should browse datasets, render overlays, and support selection and inspection before implementing drawing and mutation workflows.
- Read-only versus read-write is session state, not per-image state. It must survive image changes until the user explicitly changes it back.
- `example_data` is the default development and test dataset. All early integration and E2E flows should run against it.

## Proposed Rust Workspace Shape

Create the Rust implementation as a workspace inside this repository, not as a separate repo. Keep the Python app intact during migration.

Suggested layout:

```text
Cargo.toml
rust-toolchain.toml
crates/
  yoda-core/        # labels, geometry, parsing, rendering, shared types
  yoda-config/      # dataset yaml, color map, runtime config, defaults
  yoda-data/        # repository traits, local FS adapter, path resolution
  yoda-app/         # app state, actions, reducer/service orchestration
  yoda-web/         # axum + dioxus fullstack entrypoint
  yoda-desktop/     # dioxus desktop entrypoint
  yoda-ui/          # shared Dioxus components if separation is useful
tests/
  rust-fixtures/    # copied or generated fixtures for focused Rust tests
```

Recommended dependency boundaries:

- `yoda-core` has no Dioxus dependency.
- `yoda-config` depends on serialization and config crates, but not on UI.
- `yoda-data` depends on `yoda-core` and `yoda-config`, but not on Dioxus.
- `yoda-app` owns the state machine and orchestrates repository calls.
- `yoda-web` and `yoda-desktop` are thin composition layers.
- `yoda-ui` is optional. If the UI stays small, shared components can live in `yoda-app`; if it grows quickly, split it early.

## Recommended Crates and Libraries

- UI and runtime: `dioxus = 0.7` with `web`, `desktop`, `router`, and `fullstack` features where needed
- Web backend: `axum`, `tokio`, `tower-http`
- Serialization: `serde`, `serde_json`, `serde_yaml`
- Config: `figment` or `config-rs`, or a minimal explicit env loader if simpler
- Paths and filesystem: `camino` optionally, otherwise `std::path`
- Images: `image`
- Error handling: `thiserror`, `anyhow`
- Logging and tracing: `tracing`, `tracing-subscriber`
- Testing: built-in Rust test framework, `insta` for snapshots, `assert_fs`, `tempfile`, `tokio-test`, Playwright or Dioxus-supported browser E2E path for web

## Non-Negotiable Behavioral Contracts

These should be preserved unless consciously changed and documented:

- Missing label file means zero labels, not an error.
- YOLO normalized coordinates remain the source of truth for writing labels.
- Polygon labels require at least three points.
- Bounding box and polygon pixel geometry must be deterministic.
- Directory trees load lazily and ignore hidden files.
- Supported image extensions remain `.jpg`, `.jpeg`, `.png`, `.bmp`, `.webp`.
- Class map is loaded from Ultralytics dataset YAML, using the `names` section.
- The app starts in read-only mode by default.
- The UI exposes a lock control that is enabled by default and must be explicitly toggled before any edit operation is allowed.
- Read-only or read-write state persists across image changes within the running app session.
- Edit-capable actions must be gated in both UI state and mutation execution paths so a locked UI cannot accidentally write labels.
- Viewer-first milestone does not require edit parity.

## Data and Fixture Strategy

Use `example_data` as the first-class fixture source for development and tests.

Recommended fixture split:

1. Keep `example_data` as the integration fixture for real dataset behavior.
2. Add focused Rust test fixtures for malformed, missing, and edge-case labels so tests stay small and deterministic.
3. Copy only the minimum required images and labels into Rust-specific temp fixtures where test isolation matters.
4. Avoid mutating `example_data` in normal tests. Any mutation test should copy files to a temp directory first.

## Segment Plan

### Segment 0: Freeze the Reference Contract

Goal:
Establish the precise behavior that the Rust implementation must preserve.

Tasks:

1. Extract a concise parity checklist from the Python tests and current UI behavior.
2. Tag each behavior as `viewer-first`, `edit-phase`, or `optional polish`.
3. Record the expected dataset assumptions using `example_data/carparts-seg.yaml` and the `example_data/images` plus `example_data/labels` structure.
4. Record the new access-mode contract: app starts locked, unlock is explicit, and edit enablement persists across image changes.
5. Identify any current Python behavior that is accidental or undesirable so it does not get reintroduced by default.

Deliverables:

- A parity checklist linked to current tests and modules.
- A feature classification table for milestone planning.

Tests:

- No new Rust code yet.
- Review artifact only: all current Python test categories are accounted for.

Exit criteria:

- Every existing test-backed behavior is mapped to a Rust milestone.
- The team agrees on what is intentionally deferred to phase 2.

### Segment 1: Workspace Scaffold and Tooling

Goal:
Create the Rust workspace, baseline tooling, and a reproducible local dev path.

Tasks:

1. Add workspace `Cargo.toml` and member crates.
2. Add `rust-toolchain.toml` pinned to a stable toolchain.
3. Add formatting and lint config: `rustfmt.toml`, `clippy` policy, optional `taplo` if TOML grows.
4. Add baseline scripts or Make targets for:
   `cargo fmt`, `cargo clippy --workspace`, `cargo test --workspace`, `dx serve --desktop`, `dx serve --web`.
5. Add a top-level Rust README section describing how the Rust app coexists with the Python app during migration.

Deliverables:

- Compiling empty workspace
- Standard commands for build, test, lint, and serve

Tests:

- `cargo check --workspace`
- `cargo test --workspace` with placeholder smoke tests

Exit criteria:

- New contributors can build the empty Rust workspace from a clean checkout.
- CI shape is obvious even if not fully wired yet.

### Segment 2: Core Domain Model and Label Parser

Goal:
Port the core label behavior into a pure Rust crate with no UI dependency.

Tasks:

1. Define shared types:
   `LabelObject`, `LabelType`, `Point`, `PixelBBox`, render options, and selection state helpers.
2. Implement YOLO parser behavior for:
   polygons, bounding boxes, empty files, missing files, and malformed lines.
3. Preserve normalized coordinates separately from pixel coordinates.
4. Implement YOLO writer behavior with stable precision formatting.
5. Implement helper constructors for creating labels from pixel coordinates.
6. Implement delete and reindex behavior.

Deliverables:

- `yoda-core` with parsing and writing APIs
- Shared types ready for both web and desktop

Tests:

1. Port the current parser tests from `tests/test_label.py`.
2. Add Rust-only tests for malformed input, odd point counts, and precision edge cases.
3. Add write-read round-trip tests.

Exit criteria:

- Rust label tests fully cover the Python parser contract.
- Read-write round trips are deterministic.

### Segment 3: Geometry, Hit-Testing, and Overlay Rendering

Goal:
Finish the non-UI math and rendering primitives required by the viewer.

Tasks:

1. Port polygon point-in-polygon hit testing.
2. Port bbox hit testing.
3. Implement bounding box derivation for polygons.
4. Implement SVG overlay generation for:
   polygons, bboxes, class id labels, class name labels, invisible labels, and selected-label highlighting.
5. Keep rendering output stable enough for snapshot testing.

Deliverables:

- Rendering functions and geometry utilities in `yoda-core`

Tests:

1. Port the SVG renderer tests from `tests/test_label.py`.
2. Add snapshot tests for common overlay combinations.
3. Add targeted tests for selection styling and hidden-label filtering.

Exit criteria:

- Viewer overlays can be generated in Rust without any Dioxus UI.
- Snapshot or string tests make regressions obvious.

### Segment 4: Config and Dataset Metadata

Goal:
Port dataset YAML loading, color map handling, and runtime configuration.

Tasks:

1. Implement environment-driven runtime config for image root, label root, class info path, color map path, host, and port.
2. Implement class map loading from Ultralytics-style dataset YAML.
3. Implement color map merging with defaults and user overrides.
4. Add a default development config that points to `example_data`.

Deliverables:

- `yoda-config` crate or module with typed config APIs

Tests:

1. Port the current settings and class map tests from `tests/test_config.py`.
2. Add tests for absent files, bad YAML, and override precedence.

Exit criteria:

- Rust config behavior matches Python expectations.
- The app can boot against `example_data` without custom setup.

### Segment 5: Repository Layer and Lazy File Tree

Goal:
Implement dataset access behind a shared trait so the app state and UI do not depend on direct filesystem details.

Tasks:

1. Define repository traits for:
   listing root nodes, expanding a directory, reading image metadata, reading image bytes, loading labels, saving labels, loading class map, and loading color map.
2. Implement the local filesystem adapter.
3. Implement lazy tree placeholders or an equivalent lazy expansion contract.
4. Port image-extension filtering and hidden-file handling.
5. Add path mapping logic from image path to label path with mirrored directory rules.

Deliverables:

- `yoda-data` with a local filesystem adapter
- A stable tree-node model shared by web and desktop

Tests:

1. Port the current file-tree tests from `tests/test_fileops.py`.
2. Add repository integration tests using temporary directories.
3. Add tests that prove `example_data/images/...` resolves to the matching `example_data/labels/...` file.

Exit criteria:

- Tree expansion and dataset path resolution are correct and deterministic.
- No UI code is needed to validate repository behavior.

### Segment 6: Shared App State and Actions

Goal:
Create the state container and action model that the Dioxus UI will drive.

Tasks:

1. Define app state for:
   current image, current labels, label path, image path, hidden classes, selected label, current mode, drawing vertices, zoom, pan, overlay toggles, access mode, lock state, and status metadata.
2. Define actions or reducer-style commands for:
   loading an image, toggling overlays, changing class visibility, selecting a label, toggling lock state, toggling object visibility, changing class, deleting a label, starting drawing, adding a vertex, completing drawing, canceling drawing, zooming, panning, and fitting to screen.
3. Add explicit guards so edit actions are rejected while the app is locked, even if a UI component dispatches them accidentally.
4. Keep pure state transitions separate from repository side effects.
5. Add a service layer for async operations such as loading files or persisting labels.

Deliverables:

- `yoda-app` state machine or equivalent orchestration layer

Tests:

1. State transition tests for all viewer-first actions.
2. Tests that class visibility and object visibility interact correctly.
3. Tests that access mode persists across image changes and does not reset on image load.
4. Tests that selection clearing on delete or load still behaves correctly.
5. Tests for locked-state rejection of edit actions.
6. Tests for drawing lifecycle in phase 2, even if the UI does not expose it yet.

Exit criteria:

- Most app behavior can be exercised in unit tests without rendering a UI.

### Segment 7: Web Backend and Transport Contract

Goal:
Create the local self-hosted backend required for the web target.

Tasks:

1. Define typed endpoints or server functions for:
   tree listing, directory expansion, image metadata, image bytes, labels, class map, color map, and health.
2. Implement the backend using the repository layer rather than ad hoc handlers.
3. Add safe write behavior for label updates.
4. Return errors in a structured form the UI can render cleanly.
5. Add a small health and version endpoint for smoke testing.

Deliverables:

- `yoda-web` backend with stable API contract

Tests:

1. Integration tests against the backend using `example_data`.
2. Tests for 404 and malformed request cases.
3. Tests for saving labels into a temp dataset copy.

Exit criteria:

- The web UI can load all required viewer data through a stable typed interface.

### Segment 8: Shared Dioxus Viewer UI Skeleton

Goal:
Render the viewer shell with real data but without edit-heavy features.

Tasks:

1. Build the three-pane layout:
   left tree, center image viewport, right inspector.
2. Render the file tree with lazy expansion.
3. Render image plus SVG overlay.
4. Render toolbar toggles for segmentation, bbox, class id, class name, fit, zoom in, zoom out, reset zoom, and access-mode lock state.
5. Render class legend and object list.
6. Render status information for file name, image dimensions, object count, and mode.
7. Show the lock icon as enabled by default and visually distinguish locked versus unlocked state, even before full edit parity exists.

Deliverables:

- Shared viewer UI usable in both desktop and web targets

Tests:

1. Component-level rendering tests where feasible.
2. Smoke tests that the shell renders with fixture data.
3. Browser E2E tests for page load, tree visibility, class legend visibility, image load, object list population, overlay toggle presence, and default locked state.

Exit criteria:

- The app can browse `example_data` end to end in both desktop and web modes.

### Segment 9: Viewer Interaction Parity

Goal:
Reach the full viewer-first milestone before any editing work begins.

Tasks:

1. Add selection from image clicks using hit testing.
2. Add selection syncing between overlay and object list.
3. Add zoom and pan behavior.
4. Add fit-to-screen and reset zoom.
5. Preserve lock state across image changes and other viewer navigation.
6. Add persistence for the last-opened image and basic view preferences.
7. Add keyboard shortcuts for viewer-only actions that are low risk.

Deliverables:

- Reviewable viewer-first application on web and desktop

Tests:

1. E2E tests for selection.
2. E2E tests for zoom or fit flows where automation is stable.
3. E2E tests proving the app remains locked by default after image changes unless the user has explicitly unlocked it.
4. State tests for view persistence, selection sync, and lock-state persistence across image changes.

Exit criteria:

- The Rust app is a credible replacement for browsing and inspecting datasets.
- Team review signs off on viewer UX before edit work starts.

### Segment 10: Edit Infrastructure

Goal:
Lay the mutation path without exposing all edit features at once.

Tasks:

1. Implement atomic label writes.
2. Implement change-class mutation path.
3. Implement per-object visibility and per-class visibility state behavior.
4. Ensure all mutation paths reject writes while the app is locked.
5. Ensure all writes go through the repository abstraction and return user-visible errors.

Deliverables:

- Safe mutation path ready for UI wiring

Tests:

1. Integration tests that mutate copied dataset fixtures.
2. Tests proving selection and indices remain valid after mutation.
3. Tests for failure behavior when files are missing or unwritable.
4. Tests that locked mode blocks mutation requests even if they reach the service or backend layer.

Exit criteria:

- Edit operations are stable in backend and state layers before UI affordances expand.

### Segment 11: Edit UI Parity

Goal:
Expose the current Python edit workflows in the Rust UI.

Tasks:

1. Add the lock icon flow that toggles between read-only and read-write mode.
2. Keep edit controls disabled or inert while locked, and enable them only after unlock.
3. Add object class dropdown updates.
4. Add delete button and selected-object delete behavior.
5. Add draw mode, point capture, polygon preview, finalize, and cancel.
6. Add keyboard shortcuts for edit mode and drawing flows.
7. Add user feedback for save success, failure, invalid drawing attempts, and blocked edits while locked.

Deliverables:

- Edit-capable Rust app matching current Python behavior closely enough for migration review

Tests:

1. E2E tests for default locked startup and explicit unlock.
2. E2E tests for class change.
3. E2E tests for delete.
4. E2E tests for draw finalize and cancel.
5. E2E tests that edit controls stop working again after relocking.
6. State tests for drawing lifecycle, selection behavior, and lock gating.

Exit criteria:

- Phase 2 parity is proven by tests, not just manual behavior.

### Segment 12: Packaging, Distribution, and Cutover Readiness

Goal:
Prepare the Rust implementation for adoption without removing the Python app prematurely.

Tasks:

1. Add Linux desktop packaging and smoke validation.
2. Add Windows desktop smoke packaging if feasible.
3. Add production build instructions for the self-hosted web target.
4. Update the repo README with dual-run instructions during migration.
5. Define cutover criteria and rollback strategy.

Deliverables:

- Packaged Linux desktop app
- Documented web deployment flow
- Cutover checklist

Tests:

1. Release build validation for desktop and web.
2. Final smoke test matrix using `example_data`.
3. Basic startup and shutdown checks.

Exit criteria:

- The Rust app is deployable and can be evaluated by others without Rust-specific tribal knowledge.

## Review Gates

Do not start the next segment until the current one passes review.

Required review gates:

1. Segment 1 complete before any serious feature work.
2. Segment 5 complete before UI work depends on real dataset access.
3. Segment 9 approved before starting broad edit UI work.
4. Segment 11 approved before planning Python cutover.

## Suggested Implementation Order for PRs

Keep PRs small and scoped to one segment or a subset of one segment.

Suggested PR sequence:

1. Workspace scaffold and tooling
2. Core types plus parser
3. Geometry plus SVG rendering
4. Config and dataset metadata
5. Repository layer and tree loading
6. App state and actions
7. Web backend
8. Shared viewer UI shell
9. Viewer interaction parity
10. Edit infrastructure
11. Edit UI parity
12. Packaging and docs

## Minimum Test Matrix Per Segment

- Unit tests for pure logic in `yoda-core`, `yoda-config`, and `yoda-app`
- Integration tests for repository and backend behavior using copied fixtures from `example_data`
- Viewer-first E2E coverage for web
- Manual smoke checks for desktop until a desktop automation path is chosen

## Commands to Support Early

These should exist by the end of Segment 1 or 2:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
dx serve --desktop
dx serve --web
```

Add target-specific smoke commands later, for example:

```bash
cargo test -p yoda-core
cargo test -p yoda-data
cargo test -p yoda-web
dx bundle --desktop
dx bundle --web
```

## Risks to Watch During Implementation

1. Letting Dioxus-specific types leak into the core domain crates.
2. Coupling file I/O directly to UI event handlers.
3. Skipping snapshot coverage for SVG output and then fighting regressions later.
4. Making the web target depend on direct filesystem access from the browser.
5. Starting draw and delete workflows before the viewer interaction model is stable.
6. Mutating `example_data` directly during tests.
7. Implementing lock state only in the UI and not in the reducer or backend mutation path.

## Definition of Viewer-First Done

Viewer-first is complete when all of the following are true:

1. The web app can browse `example_data` through the Rust backend.
2. The desktop app can browse `example_data` through the same shared state and UI model.
3. The UI exposes the lock control in a default locked state.
4. Tree expansion, image loading, overlay rendering, class legend, object list, selection, and zoom basics are working.
5. Unit, integration, and E2E tests cover the viewer path adequately.
6. The team explicitly approves the viewer UX before edit work begins.

## Definition of Edit Parity Done

Edit parity is complete when all of the following are true:

1. The app starts locked by default and requires explicit unlock before edits are possible.
2. Class reassignment, deletion, and polygon drawing work on both targets when unlocked.
3. Label writes are atomic and validated.
4. Keyboard shortcuts, lock transitions, and mode transitions match the approved contract.
5. All edit workflows have automated coverage or a documented reason why not.
6. The Rust app can credibly replace the Python app for normal use on the supported targets.

## Questions to Resolve Before Coding Starts

1. Whether to split `yoda-ui` from `yoda-app` immediately or only if the shared UI grows beyond a manageable size.
2. Whether to use Dioxus server functions directly or a thinner explicit Axum API surface for dataset operations.
3. Whether desktop should use the same HTTP transport as web for consistency or use direct repository injection for lower complexity. My recommendation is direct repository injection for desktop and typed backend transport for web.
4. Whether to add a lightweight conflict-detection scheme for concurrent web writes in phase 2 or defer it until real usage proves it necessary.

---

## Segment 5A: Large-Scale Dataset Tree Architecture

**Status: Implemented (2026-05-10)**

### Problem Statement

The original lazy-loading tree had four confirmed performance bottlenecks that became
unacceptable at the target scale (40–50 k images, directory depth 3–4, 50–200 files per
leaf folder):

| # | Bottleneck | Location | Root Cause |
|---|------------|----------|------------|
| 1 | Sort allocation | `yoda-data/get_dir_children` | `to_lowercase()` called inside `sort_by` comparator – O(n log n) allocations |
| 2 | Recursive clone | `yoda-ui/replace_children` | `children.clone()` at every recursion level – exponential copies |
| 3 | Sequential startup | `yoda-ui/App` startup effect | Three independent API calls awaited serially |
| 4 | Per-folder roundtrip | `yoda-ui/App` ontoggle handler | One HTTP call per folder expansion – N calls for N folders |

Bottleneck 4 is the architectural root cause.  At 50 k images spread across ~500
folders, a user expanding a deep path required hundreds of round-trips.

### Design

**Data layer (`yoda-data`)**

Two new types parallel the existing `TreeNode`/`NodeIcon` types:

- `NodeKind` — `Folder | Image` (no `Placeholder` needed; all data is loaded upfront)
- `FlatNode { id: u32, parent_id: Option<u32>, name: String, kind: NodeKind, path: String }`
- `FlatIndex { nodes: Vec<FlatNode>, image_count: usize }`

`FlatNode::id` equals the node's index in `FlatIndex::nodes`, giving O(1) lookup.

`scan_dataset_tree(root: &Path) -> FlatIndex` performs a synchronous depth-first walk
using `std::fs::read_dir`.  Within each directory, entries are sorted with pre-computed
lowercase sort keys (folders before images) to avoid the O(n log n) allocation bottleneck.

**Backend (`yoda-web`)**

`BackendState` gains a `flat_index: Arc<FlatIndex>` field populated synchronously in
`BackendState::from_settings` (called once at server startup).  Two new endpoints are
registered:

- `GET /api/tree/status` → `TreeStatusResponse { node_count, image_count }` — lightweight
   health/progress probe (always ready since the scan is synchronous).
- `GET /api/tree/flat` → `FlatIndexResponse { nodes: Vec<FlatNode>, image_count }` —
   transfers the entire flat index in a single HTTP response.

The legacy `/api/tree` and `/api/tree/children` endpoints are retained for the
server-side fallback viewer.

**UI (`yoda-ui`)**

- `Vec<TreeNode>` signal replaced by `Vec<FlatNode>` signal (`flat_nodes`).
- `BTreeSet<String>` expanded-dirs signal replaced by `BTreeSet<u32>` (node IDs).
- On startup, the UI fetches `/api/tree/flat` once.  No further tree network calls
   are ever made.
- Two reactive memos provide the tree view:
   - `children_map: Memo<HashMap<Option<u32>, Vec<u32>>>` — rebuilds only when `flat_nodes`
      changes (i.e., once at startup).
   - `visible_rows: Memo<Vec<VisibleRow>>` — reruns when `children_map` or `expanded_dirs`
      changes.  O(visible nodes) per expand/collapse; fully client-side.
- `TreeNodeView` (recursive Dioxus component with async expand) replaced by `FlatNodeView`
   (flat component, no async, no network call on toggle).
- `replace_children`, `node_needs_children`, `fetch_tree_root`, `fetch_tree_children`
   removed.

### Scaling Characteristics

| Operation | Before | After |
|-----------|--------|-------|
| Initial tree load | 1 RTT (roots only) | 1 RTT (full index) |
| Expand a folder | 1 RTT per folder | 0 RTT (memo recompute) |
| Expand N folders | N RTTs | 0 RTTs |
| DOM nodes rendered | Only roots until expanded | Only visible (expanded) rows |
| Children map rebuild | On every expand | Only when flat_nodes changes |
| Sort per directory | O(n log n) allocations | O(n) allocations (pre-keyed) |

For 50 k images across 500 folders, the worst-case initial transfer is ~50 k × ~80 bytes
≈ 4 MB of JSON.  This is acceptable for a LAN/localhost tool.

### Non-Negotiable Contract Update

> "Directory trees are fully scanned at startup into a server-side cache and transferred
> to the client in a single request; browsing and expansion are then fully local.
> Hidden files (names starting with `.`) are excluded."

The old contract ("lazy loading") is superseded by the above.

### Files Changed

| File | Change |
|------|--------|
| `crates/yoda-data/src/lib.rs` | Added `NodeKind`, `FlatNode`, `FlatIndex`; added `scan_dataset_tree` + `scan_dir_flat`; fixed sort in `get_dir_children` (pre-computed keys) |
| `crates/yoda-web/src/lib.rs` | Added `flat_index` to `BackendState`; added `TreeStatusResponse`, `FlatIndexResponse`, `tree_status`, `tree_flat` handlers; registered new routes |
| `crates/yoda-ui/src/lib.rs` | Replaced tree model with flat index; added `VisibleRow`, `build_children_map`, `compute_visible_rows`; added `children_map` and `visible_rows` memos; replaced `TreeNodeView` with `FlatNodeView`; removed old lazy helpers |

### Tests Added

- `yoda-data::scan_tests::scan_counts_images_and_skips_hidden` — counts images, excludes hidden
- `yoda-data::scan_tests::scan_id_matches_index` — node ID equals vec index invariant
- `yoda-data::scan_tests::scan_folders_before_images` — sort order correct