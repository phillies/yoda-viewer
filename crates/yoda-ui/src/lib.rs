use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use dioxus::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use yoda_app::{
    AccessMode, AppAction, AppEffect, AppState, InteractionMode, LoadedImage,
    apply_action,
};
use yoda_core::{LabelObject, LabelType, Point, RenderOptions, render_labels_to_svg};
use yoda_data::{FilterMode, FlatNode, NodeIcon, NodeKind};

pub const PAN_ZOOM_SCRIPT: &str = r#"
(function () {
    if (window.__yodaPanZoomController) {
        return;
    }

    const controller = {
        container: null,
        stage: null,
        zoom: 1,
        panX: 0,
        panY: 0,
        dragging: false,
        lastX: 0,
        lastY: 0,
        imageKey: "",
        containerObserver: null,
        documentObserver: null,
        cleanup: null,
    };
    window.__yodaPanZoomController = controller;

    function clamp(value, min, max) {
        return Math.min(max, Math.max(min, value));
    }

    function updateStage() {
        if (!controller.container || !document.contains(controller.container)) {
            controller.stage = null;
            return false;
        }

        controller.stage = controller.container.querySelector('[data-panzoom-stage="true"]');
        if (!controller.stage) {
            controller.container.style.cursor = 'default';
            return false;
        }

        const nextKey = controller.stage.getAttribute('data-image-key') || '';
        if (nextKey !== controller.imageKey) {
            controller.imageKey = nextKey;
            controller.zoom = 1;
            controller.panX = 0;
            controller.panY = 0;
        }

        return true;
    }

    function applyTransform() {
        if (!controller.stage && !updateStage()) {
            return;
        }

        controller.stage.style.transform = `translate(${controller.panX}px, ${controller.panY}px) scale(${controller.zoom})`;
        controller.container.dataset.zoom = controller.zoom.toFixed(2);
        controller.container.style.cursor = controller.dragging ? 'grabbing' : 'grab';
    }

    function stopDragging() {
        controller.dragging = false;
        if (controller.container) {
            controller.container.style.cursor = controller.stage ? 'grab' : 'default';
        }
    }

    function resetView() {
        controller.zoom = 1;
        controller.panX = 0;
        controller.panY = 0;
        applyTransform();
    }

    function handleContainerMutations() {
        updateStage();
        applyTransform();
    }

    function bindContainer(container) {
        if (controller.container === container) {
            handleContainerMutations();
            return;
        }

        if (controller.cleanup) {
            controller.cleanup();
            controller.cleanup = null;
        }

        controller.container = container;
        controller.stage = null;
        controller.imageKey = '';
        stopDragging();

        const onWheel = function (event) {
            if (!updateStage()) {
                return;
            }

            event.preventDefault();
            const rect = controller.container.getBoundingClientRect();
            const pointerX = event.clientX - rect.left;
            const pointerY = event.clientY - rect.top;
            const nextZoom = clamp(controller.zoom * (event.deltaY < 0 ? 1.1 : 0.9), 0.25, 6.0);
            const ratio = nextZoom / controller.zoom;

            controller.panX = pointerX - (pointerX - controller.panX) * ratio;
            controller.panY = pointerY - (pointerY - controller.panY) * ratio;
            controller.zoom = nextZoom;
            applyTransform();
        };

        const onMouseDown = function (event) {
            if (event.button !== 0 || !updateStage()) {
                return;
            }
            // Skip drag in polygon-draw mode – clicks are handled by the SVG draw overlay.
            if (controller.container && controller.container.dataset.drawMode === 'true') {
                return;
            }

            controller.dragging = true;
            controller.lastX = event.clientX;
            controller.lastY = event.clientY;
            applyTransform();
            event.preventDefault();
        };

        const onMouseMove = function (event) {
            if (!controller.dragging || !controller.stage) {
                return;
            }

            controller.panX += event.clientX - controller.lastX;
            controller.panY += event.clientY - controller.lastY;
            controller.lastX = event.clientX;
            controller.lastY = event.clientY;
            applyTransform();
        };

        const onDoubleClick = function () {
            // In edit mode, double-click is reserved for label selection — skip view reset.
            if (controller.container && controller.container.dataset.editMode === 'unlocked') {
                return;
            }
            if (!updateStage()) {
                return;
            }

            resetView();
        };

        controller.container.addEventListener('wheel', onWheel, { passive: false });
        controller.container.addEventListener('mousedown', onMouseDown);
        controller.container.addEventListener('dblclick', onDoubleClick);
        window.addEventListener('mousemove', onMouseMove);
        window.addEventListener('mouseup', stopDragging);
        window.addEventListener('mouseleave', stopDragging);

        controller.containerObserver = new MutationObserver(handleContainerMutations);
        controller.containerObserver.observe(controller.container, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-image-key'] });

        controller.cleanup = function () {
            stopDragging();
            controller.containerObserver?.disconnect();
            controller.container.removeEventListener('wheel', onWheel);
            controller.container.removeEventListener('mousedown', onMouseDown);
            controller.container.removeEventListener('dblclick', onDoubleClick);
            window.removeEventListener('mousemove', onMouseMove);
            window.removeEventListener('mouseup', stopDragging);
            window.removeEventListener('mouseleave', stopDragging);
        };

        handleContainerMutations();
    }

    function scanForContainer() {
        const nextContainer = document.querySelector('[data-panzoom-container="true"]');
        if (!nextContainer) {
            return false;
        }

        bindContainer(nextContainer);
        return true;
    }

    function start() {
        scanForContainer();
        controller.documentObserver = new MutationObserver(scanForContainer);
        controller.documentObserver.observe(document.documentElement, { childList: true, subtree: true });
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start, { once: true });
    } else {
        start();
    }
})();
"#;

pub const DRAW_SCRIPT: &str = r#"
(function () {
    if (window.__yodaDrawSetup) { return; }
    window.__yodaDrawSetup = true;

    // Route keyboard shortcuts to hidden Dioxus buttons so they work without
    // requiring explicit focus on any particular element.
    document.addEventListener('keydown', function (event) {
        const tag = ((event.target || {}).tagName || '').toUpperCase();
        if (tag === 'INPUT' || tag === 'SELECT' || tag === 'TEXTAREA') { return; }

        if (event.key === 'Delete' || event.key === 'Backspace') {
            const btn = document.getElementById('yoda-key-delete');
            if (btn) { event.preventDefault(); btn.click(); }
        } else if (event.key === 'Escape') {
            const btn = document.getElementById('yoda-key-escape');
            if (btn) { event.preventDefault(); btn.click(); }
        } else if (event.key === 'Enter') {
            const btn = document.getElementById('yoda-key-enter');
            if (btn) { event.preventDefault(); btn.click(); }
        }
    });
})();
"#;

const APP_CSS: &str = r#"
:root {
    color-scheme: dark;
    --bg: #161611;
    --panel: #242118;
    --panel-2: #2f2a1d;
    --panel-3: #3a3424;
    --line: #585038;
    --text: #e9dfc8;
    --muted: #b8ad90;
    --accent: #7f9f4b;
    --accent-soft: rgba(127, 159, 75, 0.22);
    --good: #9fbe65;
    --warn: #cf9c5a;
}
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; background: var(--bg); color: var(--text); font-family: "Segoe UI", "Inter", sans-serif; }
body { overflow: hidden; }
button, select { font: inherit; }
.app-shell { display: grid; grid-template-columns: 280px minmax(0, 1fr) 320px; height: 100vh; gap: 0; }
.panel { border-right: 1px solid var(--line); background: rgba(36, 33, 24, 0.9); backdrop-filter: blur(14px); min-height: 0; }
.panel.right { border-right: none; border-left: 1px solid var(--line); }
.panel-inner { display: flex; flex-direction: column; height: 100%; min-height: 0; }
.section-title { padding: 16px 18px 10px; font-size: 11px; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); }
.tree-scroll, .side-scroll { overflow: auto; min-height: 0; padding: 0 10px 14px; }
.classes-scroll, .objects-scroll { overflow: auto; min-height: 0; max-height: 50%; padding: 0 10px 14px; }
.tree-node { margin-left: var(--indent); }
.tree-row { width: 100%; display: flex; align-items: center; gap: 8px; border: 1px solid transparent; background: transparent; color: inherit; padding: 8px 10px; border-radius: 10px; text-align: left; cursor: pointer; }
.tree-row:hover { background: rgba(233,223,200,0.06); border-color: rgba(233,223,200,0.08); }
.tree-row.selected { background: var(--accent-soft); border-color: rgba(127, 159, 75, 0.42); }
.tree-arrow { width: 14px; text-align: center; color: var(--muted); }
.tree-icon { width: 18px; height: 18px; color: var(--muted); flex: none; }
.tree-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.content { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
.toolbar { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; padding: 14px 16px; border-bottom: 1px solid var(--line); background: rgba(22, 22, 17, 0.82); backdrop-filter: blur(10px); }
.toolbar-group { display: inline-flex; gap: 8px; align-items: center; padding: 6px; border-radius: 12px; background: rgba(233,223,200,0.04); border: 1px solid rgba(233,223,200,0.08); }
.toolbar button, .toolbar select { background: var(--panel-2); color: var(--text); border: 1px solid var(--line); border-radius: 10px; padding: 8px 12px; }
.toolbar button.active { background: var(--accent); border-color: var(--accent); }
.toolbar button:disabled, .toolbar select:disabled { opacity: 0.45; cursor: not-allowed; }
.status-pill { padding: 8px 12px; border-radius: 999px; font-size: 12px; border: 1px solid var(--line); background: rgba(233,223,200,0.04); }
.status-pill.locked { color: var(--warn); }
.status-pill.unlocked { color: var(--good); }
.viewport-wrap { flex: 1; min-height: 0; padding: 18px; overflow: hidden; }
.viewport { min-height: 100%; border: 1px solid var(--line); border-radius: 22px; background: var(--panel); display: flex; align-items: center; justify-content: center; padding: 18px; position: relative; }
.canvas-stage { transform-origin: top left; will-change: transform; }
.canvas { position: relative; display: inline-block; line-height: 0; max-width: 100%; }
.canvas img.main-image { display: block; max-width: min(100%, 1100px); max-height: calc(100vh - 220px); border-radius: 18px; box-shadow: 0 30px 80px rgba(0,0,0,0.45); }
.canvas img.overlay-image { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
.empty-state { max-width: 420px; text-align: center; color: var(--muted); }
.empty-state h2 { margin: 0 0 8px; color: var(--text); font-size: 28px; }
.empty-state p { margin: 0; line-height: 1.5; }
.status-bar { display: flex; flex-wrap: wrap; gap: 12px; padding: 12px 16px; border-top: 1px solid var(--line); color: var(--muted); background: rgba(22, 22, 17, 0.82); font-size: 12px; }
.legend-item, .object-row { display: flex; align-items: center; gap: 6px; padding: 5px 8px; margin-bottom: 4px; border-radius: 8px; background: rgba(233,223,200,0.04); border: 1px solid transparent; font-size: 12px; }
.object-row.selected { border-color: rgba(127, 159, 75, 0.42); background: var(--accent-soft); }
.swatch { width: 8px; height: 8px; border-radius: 999px; flex: none; }
.legend-name, .object-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
.legend-item button, .object-row button, .object-row select { background: var(--panel-2); color: var(--text); border: 1px solid var(--line); border-radius: 6px; padding: 3px 7px; font-size: 11px; }
.object-row button.ghost { background: transparent; }
.object-row button.delete { color: #ff8e8e; }
.stack-gap { height: 4px; }
.message { margin: 8px 16px 0; padding: 10px 12px; border-radius: 12px; font-size: 13px; }
.message.error { background: rgba(255, 88, 88, 0.12); border: 1px solid rgba(255, 88, 88, 0.2); color: #ff9f9f; }
.message.info { background: rgba(127, 159, 75, 0.18); border: 1px solid rgba(127, 159, 75, 0.34); color: #d8e7ba; }
@media (max-width: 1180px) {
  .app-shell { grid-template-columns: 240px minmax(0, 1fr); grid-template-rows: minmax(0, 1fr) 260px; }
  .panel.right { grid-column: 1 / span 2; border-left: none; border-top: 1px solid var(--line); }
}
@media (max-width: 860px) {
  .app-shell { grid-template-columns: 1fr; grid-template-rows: 220px minmax(0, 1fr) 260px; }
  .panel { border-right: none; border-bottom: 1px solid var(--line); }
  .panel.right { border-top: 1px solid var(--line); }
  .canvas img.main-image { max-height: 50vh; }
}
.filter-bar { padding: 8px 10px 4px; border-bottom: 1px solid var(--line); }
.filter-bar-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 6px; }
.filter-label { font-size: 11px; letter-spacing: 0.12em; text-transform: uppercase; color: var(--muted); }
.filter-clear { background: transparent; color: var(--warn); border: 1px solid rgba(207,156,90,0.3); border-radius: 8px; padding: 3px 8px; font-size: 11px; cursor: pointer; }
.filter-clear:hover { background: rgba(207,156,90,0.12); }
.filter-mode { display: flex; gap: 10px; margin-bottom: 6px; font-size: 12px; color: var(--muted); }
.filter-mode label { display: flex; align-items: center; gap: 4px; cursor: pointer; }
.filter-chips { display: flex; flex-wrap: wrap; gap: 5px; }
.filter-chip { display: inline-flex; align-items: center; gap: 6px; background: rgba(233,223,200,0.04); border: 1px solid var(--line); border-radius: 20px; padding: 4px 10px; font-size: 12px; cursor: pointer; color: inherit; }
.filter-chip:hover { background: rgba(233,223,200,0.08); }
.filter-chip.active { border-color: var(--accent); background: var(--accent-soft); }
.chip-swatch { width: 8px; height: 8px; border-radius: 50%; flex: none; }
.filter-active-badge { color: var(--accent); border-color: rgba(127, 159, 75, 0.42); }
.draw-hint { color: var(--muted); font-size: 12px; padding: 4px 8px; border-radius: 8px; background: rgba(233,223,200,0.04); border: 1px dashed var(--line); }
.draw-vertex-count { color: var(--good); font-size: 12px; padding: 4px 8px; font-variant-numeric: tabular-nums; }
"#;

/// Radius in image-pixel space of the "click here to close" zone on the first draw vertex.
const CLOSE_ZONE_RADIUS: f32 = 18.0;

#[derive(Debug, Clone, Deserialize)]
struct LabelsResponse {
    image_path: String,
    label_path: String,
    width: u32,
    height: u32,
    labels: Vec<LabelObject>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClassMapResponse {
    class_map: HashMap<u32, String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ColorMapResponse {
    color_map: HashMap<u32, [u8; 3]>,
}

#[derive(Debug, Clone, Deserialize)]
struct ClassIndexResponse {
    entries: HashMap<String, Vec<u32>>,
    #[allow(dead_code)]
    all_class_ids: Vec<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct SaveLabelsRequest {
    labels: Vec<LabelObject>,
}

/// A pre-computed, flat row ready for rendering in the tree panel.
/// Only nodes currently visible (root level + inside expanded folders) are
/// included.  Cloned cheaply; fields kept intentionally small.
#[derive(Debug, Clone, PartialEq)]
struct VisibleRow {
    id: u32,
    name: String,
    kind: NodeKind,
    /// Absolute path string – passed directly to the image viewer.
    path: String,
    /// Visual indent level (0 = root).
    depth: usize,
    /// `true` when this is a folder that contains at least one child.
    has_children: bool,
    /// `true` when this folder is currently expanded.
    is_expanded: bool,
}

/// Response shape returned by `/api/tree/flat`.
#[derive(Debug, Clone, Deserialize)]
struct FlatIndexResponse {
    nodes: Vec<FlatNode>,
    #[allow(dead_code)]
    image_count: usize,
}

/// Build a `parent_id → [child_id, …]` lookup map from the flat node list.
/// The map is keyed by `Option<u32>` where `None` represents root-level nodes.
fn build_children_map(nodes: &[FlatNode]) -> HashMap<Option<u32>, Vec<u32>> {
    let mut map: HashMap<Option<u32>, Vec<u32>> = HashMap::new();
    for node in nodes {
        map.entry(node.parent_id).or_default().push(node.id);
    }
    map
}

fn collect_visible_rows(
    parent_id: Option<u32>,
    depth: usize,
    nodes: &[FlatNode],
    children_map: &HashMap<Option<u32>, Vec<u32>>,
    expanded: &std::collections::BTreeSet<u32>,
    result: &mut Vec<VisibleRow>,
) {
    let Some(children_ids) = children_map.get(&parent_id) else { return };
    for &child_id in children_ids {
        let Some(node) = nodes.get(child_id as usize) else { continue };
        let is_expanded = node.kind == NodeKind::Folder && expanded.contains(&child_id);
        let has_children =
            node.kind == NodeKind::Folder && children_map.contains_key(&Some(child_id));
        result.push(VisibleRow {
            id: child_id,
            name: node.name.clone(),
            kind: node.kind,
            path: node.path.clone(),
            depth,
            has_children,
            is_expanded,
        });
        if is_expanded {
            collect_visible_rows(
                Some(child_id),
                depth + 1,
                nodes,
                children_map,
                expanded,
                result,
            );
        }
    }
}

/// Compute the ordered list of rows that should be rendered in the tree panel.
fn compute_visible_rows(
    nodes: &[FlatNode],
    children_map: &HashMap<Option<u32>, Vec<u32>>,
    expanded: &std::collections::BTreeSet<u32>,
) -> Vec<VisibleRow> {
    let mut result = Vec::new();
    collect_visible_rows(None, 0, nodes, children_map, expanded, &mut result);
    result
}

/// Collect rows for the filtered tree view.
///
/// When a class filter is active, only folders that contain matching images
/// (directly or transitively) are shown. Empty folders are hidden. Expand/collapse
/// state is respected: children are only rendered for expanded folders.
fn collect_filtered_rows(
    parent_id: Option<u32>,
    depth: usize,
    nodes: &[FlatNode],
    children_map: &HashMap<Option<u32>, Vec<u32>>,
    matching_images: &std::collections::HashSet<u32>,
    matching_folders: &std::collections::HashSet<u32>,
    expanded_dirs: &std::collections::BTreeSet<u32>,
    result: &mut Vec<VisibleRow>,
) {
    let Some(children_ids) = children_map.get(&parent_id) else { return };
    for &child_id in children_ids {
        let Some(node) = nodes.get(child_id as usize) else { continue };
        match node.kind {
            NodeKind::Image => {
                if matching_images.contains(&child_id) {
                    result.push(VisibleRow {
                        id: child_id,
                        name: node.name.clone(),
                        kind: NodeKind::Image,
                        path: node.path.clone(),
                        depth,
                        has_children: false,
                        is_expanded: false,
                    });
                }
            }
            NodeKind::Folder => {
                if matching_folders.contains(&child_id) {
                    let has_children = children_map.contains_key(&Some(child_id));
                    let is_expanded = expanded_dirs.contains(&child_id);
                    result.push(VisibleRow {
                        id: child_id,
                        name: node.name.clone(),
                        kind: NodeKind::Folder,
                        path: node.path.clone(),
                        depth,
                        has_children,
                        is_expanded,
                    });
                    if is_expanded {
                        collect_filtered_rows(
                            Some(child_id),
                            depth + 1,
                            nodes,
                            children_map,
                            matching_images,
                            matching_folders,
                            expanded_dirs,
                            result,
                        );
                    }
                }
            }
        }
    }
}

/// Compute the tree rows shown when a class filter is active.
fn compute_filtered_rows(
    nodes: &[FlatNode],
    children_map: &HashMap<Option<u32>, Vec<u32>>,
    class_index: &HashMap<String, Vec<u32>>,
    filter_classes: &BTreeSet<u32>,
    filter_mode: FilterMode,
    expanded_dirs: &std::collections::BTreeSet<u32>,
) -> Vec<VisibleRow> {
    use std::collections::HashSet;

    // Collect matching image node IDs.
    let matching_image_ids: HashSet<u32> = nodes
        .iter()
        .filter(|node| {
            if node.kind != NodeKind::Image {
                return false;
            }
            let Some(class_ids) = class_index.get(&node.path) else {
                return false;
            };
            match filter_mode {
                FilterMode::Any => {
                    class_ids.iter().any(|id| filter_classes.contains(id))
                }
                FilterMode::All => {
                    filter_classes.iter().all(|id| class_ids.contains(id))
                }
            }
        })
        .map(|n| n.id)
        .collect();

    // Collect all ancestor folder IDs for those images.
    let mut matching_folder_ids: HashSet<u32> = HashSet::new();
    for &image_id in &matching_image_ids {
        let mut current = image_id;
        loop {
            let Some(node) = nodes.get(current as usize) else { break };
            match node.parent_id {
                Some(parent_id) => {
                    if !matching_folder_ids.insert(parent_id) {
                        break; // already processed this ancestor chain
                    }
                    current = parent_id;
                }
                None => break,
            }
        }
    }

    let mut result = Vec::new();
    collect_filtered_rows(
        None,
        0,
        nodes,
        children_map,
        &matching_image_ids,
        &matching_folder_ids,
        expanded_dirs,
        &mut result,
    );
    result
}

#[component]
pub fn RootApp() -> Element {
    rsx! {
        App {}
    }
}

#[component]
pub fn App(api_base: Option<String>) -> Element {
    let api_base = use_signal(|| api_base.unwrap_or_default());
    let app_state = use_signal(AppState::default);
    let flat_nodes = use_signal(Vec::<FlatNode>::new);
    let mut expanded_dirs = use_signal(std::collections::BTreeSet::<u32>::new);
    let color_map = use_signal(HashMap::<u32, (u8, u8, u8)>::new);
    let mut selected_image_path = use_signal(|| None::<String>);
    let tree_loading = use_signal(|| true);
    let image_loading = use_signal(|| false);

    {
        let api_base = api_base;
        let mut flat_nodes = flat_nodes;
        let mut tree_loading = tree_loading;
        let mut color_map = color_map;
        let app_state = app_state;
        use_effect(move || {
            let api_base_value = api_base();
            spawn(async move {
                tree_loading.set(true);
                match fetch_flat_index(&api_base_value).await {
                    Ok(nodes) => flat_nodes.set(nodes),
                    Err(error) => set_status_error(app_state, error),
                }

                match fetch_class_map(&api_base_value).await {
                    Ok(class_map) => {
                        let effects = reduce_state(
                            app_state,
                            AppAction::ClassMapLoaded(class_map),
                        );
                        run_effects(api_base_value.clone(), app_state, effects);
                    }
                    Err(error) => set_status_error(app_state, error),
                }

                match fetch_color_map(&api_base_value).await {
                    Ok(map) => color_map.set(map),
                    Err(error) => set_status_error(app_state, error),
                }

                match fetch_class_index(&api_base_value).await {
                    Ok(entries) => {
                        let effects = reduce_state(
                            app_state,
                            AppAction::ClassIndexLoaded(entries),
                        );
                        run_effects(api_base_value.clone(), app_state, effects);
                    }
                    Err(error) => set_status_error(app_state, error),
                }

                tree_loading.set(false);
            });
        });
    }

    {
        let api_base = api_base;
        let app_state = app_state;
        let selected_image_path = selected_image_path;
        let mut image_loading = image_loading;
        use_effect(move || {
            let Some(image_path) = selected_image_path() else {
                return;
            };

            let api_base_value = api_base();
            spawn(async move {
                image_loading.set(true);
                match fetch_labels(&api_base_value, &image_path).await {
                    Ok(response) => {
                        let loaded = LoadedImage {
                            image_path: PathBuf::from(&response.image_path),
                            label_path: PathBuf::from(&response.label_path),
                            image_dimensions: yoda_app::ImageDimensions {
                                width: response.width,
                                height: response.height,
                            },
                            labels: response.labels,
                        };
                        let effects =
                            reduce_state(app_state, AppAction::ImageLoaded(loaded));
                        run_effects(api_base_value.clone(), app_state, effects);
                    }
                    Err(error) => set_status_error(app_state, error),
                }
                image_loading.set(false);
            });
        });
    }

    // children_map rebuilds only when flat_nodes changes; visible_rows/filtered_rows rebuild
    // when nodes, expanded state, or filter changes. All are pure client-side, no I/O.
    let children_map = use_memo(move || build_children_map(&flat_nodes.read()));
    let visible_rows = use_memo(move || {
        let nodes = flat_nodes.read();
        let map = children_map.read();
        let expanded = expanded_dirs.read();
        let state = app_state();
        if state.filter_classes.is_empty() {
            compute_visible_rows(&nodes, &map, &expanded)
        } else {
            compute_filtered_rows(
                &nodes,
                &map,
                &state.class_index,
                &state.filter_classes,
                state.filter_mode,
                &expanded,
            )
        }
    });

    let state_value = app_state();
    let visible_labels = state_value.visible_labels();
    let overlay_data_uri =
        render_overlay_data_uri(&state_value, &color_map(), &visible_labels);
    let selected_image_src = state_value
        .current_image_path
        .as_ref()
        .map(|path| build_image_url(&api_base(), &path.to_string_lossy()));
    let image_stage_key = state_value
        .current_image_path
        .as_ref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| String::from("no-image"));
    let selected_image_path_value = selected_image_path();
    let class_ids = sorted_class_ids(&state_value.class_map, &visible_labels);
    let class_options = class_ids
        .iter()
        .copied()
        .map(|class_id| {
            (
                class_id,
                state_value
                    .class_map
                    .get(&class_id)
                    .cloned()
                    .unwrap_or_else(|| format!("class {class_id}")),
            )
        })
        .collect::<Vec<_>>();
    let status_class = if state_value.is_locked() {
        "status-pill locked"
    } else {
        "status-pill unlocked"
    };
    let is_locked = state_value.is_locked();
    let is_unlocked = !is_locked;
    let image_name = state_value
        .current_image_path
        .as_ref()
        .and_then(|path| path.file_name().and_then(|name| name.to_str()))
        .unwrap_or("None")
        .to_string();
    let dimensions_text = state_value
        .image_dimensions
        .map(|dims| format!("{} x {}", dims.width, dims.height))
        .unwrap_or_else(|| String::from("-"));
    let mode_text = if state_value.is_locked() { "Viewer" } else { "Edit" };

    rsx! {
        document::Style { "{APP_CSS}" }
        script { "{PAN_ZOOM_SCRIPT}" }
        script { "{DRAW_SCRIPT}" }
        main { class: "app-shell",
            aside { class: "panel",
                div { class: "panel-inner",
                    div { class: "section-title", "Dataset" }
                    ClassFilterBar {
                        class_map: state_value.class_map.clone(),
                        color_map: color_map(),
                        filter_classes: state_value.filter_classes.clone(),
                        filter_mode: state_value.filter_mode,
                        ontoggle_class: move |class_id: u32| {
                            let selected = !app_state().filter_classes.contains(&class_id);
                            let effects = reduce_state(app_state, AppAction::SetFilterClass { class_id, selected });
                            run_effects(api_base(), app_state, effects);
                        },
                        onset_mode: move |mode: FilterMode| {
                            let effects = reduce_state(app_state, AppAction::SetFilterMode(mode));
                            run_effects(api_base(), app_state, effects);
                        },
                        onclear: move |_| {
                            let effects = reduce_state(app_state, AppAction::ClearClassFilter);
                            run_effects(api_base(), app_state, effects);
                        },
                    }
                    if tree_loading() {
                        div { class: "message info", "Loading dataset tree..." }
                    }
                    div { class: "tree-scroll",
                        for row in visible_rows().into_iter() {
                            FlatNodeView {
                                key: "{row.id}",
                                row,
                                selected_image_path: selected_image_path_value.clone(),
                                ontoggle: move |node_id: u32| {
                                    let mut dirs = expanded_dirs.write();
                                    if dirs.contains(&node_id) {
                                        dirs.remove(&node_id);
                                    } else {
                                        dirs.insert(node_id);
                                    }
                                },
                                onselect: move |image_path: String| {
                                    selected_image_path.set(Some(image_path));
                                },
                            }
                        }
                    }
                }
            }

            section { class: "content",
                div { class: "toolbar",
                    div { class: "toolbar-group",
                        button {
                            class: if state_value.show_segmask { "active" } else { "" },
                            onclick: move |_| {
                                let effects = reduce_state(app_state, AppAction::ToggleSegmask);
                                run_effects(api_base(), app_state, effects);
                            },
                            "Mask"
                        }
                        button {
                            class: if state_value.show_bbox { "active" } else { "" },
                            onclick: move |_| {
                                let effects = reduce_state(app_state, AppAction::ToggleBbox);
                                run_effects(api_base(), app_state, effects);
                            },
                            "BBox"
                        }
                        button {
                            class: if state_value.show_class_id { "active" } else { "" },
                            onclick: move |_| {
                                let effects = reduce_state(app_state, AppAction::ToggleClassId);
                                run_effects(api_base(), app_state, effects);
                            },
                            "Class ID"
                        }
                        button {
                            class: if state_value.show_class_name { "active" } else { "" },
                            onclick: move |_| {
                                let effects = reduce_state(app_state, AppAction::ToggleClassName);
                                run_effects(api_base(), app_state, effects);
                            },
                            "Class Name"
                        }
                    }
                    div { class: "toolbar-group",
                        button {
                            onclick: move |_| {
                                let next_mode = if app_state().is_locked() { AccessMode::Unlocked } else { AccessMode::Locked };
                                let effects = reduce_state(app_state, AppAction::SetAccessMode(next_mode));
                                run_effects(api_base(), app_state, effects);
                            },
                            if state_value.is_locked() { "Unlock Editing" } else { "Lock Editing" }
                        }
                        span { class: status_class, if state_value.is_locked() { "Locked" } else { "Unlocked" } }
                    }
                    if is_unlocked {
                        div { class: "toolbar-group",
                            button {
                                class: if state_value.interaction_mode == InteractionMode::Draw { "active" } else { "" },
                                onclick: move |_| {
                                    let next = if app_state().interaction_mode == InteractionMode::Draw {
                                        InteractionMode::Edit
                                    } else {
                                        InteractionMode::Draw
                                    };
                                    let effects = reduce_state(app_state, AppAction::SetInteractionMode(next));
                                    run_effects(api_base(), app_state, effects);
                                },
                                "Draw Polygon"
                            }
                            select {
                                value: "{state_value.drawing_class_id}",
                                onchange: move |event| {
                                    if let Ok(class_id) = event.value().parse::<u32>() {
                                        let effects = reduce_state(app_state, AppAction::SetDrawClassId(class_id));
                                        run_effects(api_base(), app_state, effects);
                                    }
                                },
                                for (class_id, label) in class_options.iter().cloned() {
                                    option { value: "{class_id}", "{label}" }
                                }
                            }
                        }
                        if state_value.interaction_mode == InteractionMode::Draw {
                            span { class: "draw-vertex-count",
                                "{state_value.drawing_vertices.len()} pts"
                            }
                            button {
                                disabled: state_value.drawing_vertices.len() < 3,
                                onclick: move |_| {
                                    let effects = reduce_state(app_state, AppAction::FinishDrawing);
                                    run_effects(api_base(), app_state, effects);
                                },
                                "Finish (Enter)"
                            }
                            button {
                                onclick: move |_| {
                                    let effects = reduce_state(app_state, AppAction::CancelDrawing);
                                    run_effects(api_base(), app_state, effects);
                                },
                                "Cancel (Esc)"
                            }
                        }
                    }
                    if image_loading() {
                        span { class: "status-pill", "Loading image..." }
                    }
                    // Hidden virtual buttons for global keyboard shortcuts (clicked by DRAW_SCRIPT).
                    button {
                        id: "yoda-key-delete",
                        style: "display:none;",
                        onclick: move |_| {
                            let effects = reduce_state(app_state, AppAction::DeleteSelectedLabel);
                            run_effects(api_base(), app_state, effects);
                        }
                    }
                    button {
                        id: "yoda-key-escape",
                        style: "display:none;",
                        onclick: move |_| {
                            let effects = reduce_state(app_state, AppAction::CancelDrawing);
                            run_effects(api_base(), app_state, effects);
                        }
                    }
                    button {
                        id: "yoda-key-enter",
                        style: "display:none;",
                        onclick: move |_| {
                            let effects = reduce_state(app_state, AppAction::FinishDrawing);
                            run_effects(api_base(), app_state, effects);
                        }
                    }
                }

                if let Some(error_text) = state_value.status.error_text.clone() {
                    div { class: "message error", "{error_text}" }
                }
                if let Some(status_text) = state_value.status.status_text.clone() {
                    div { class: "message info", "{status_text}" }
                }

                div { class: "viewport-wrap",
                    div {
                        class: "viewport",
                        "data-panzoom-container": "true",
                        "data-edit-mode": if is_locked { "locked" } else { "unlocked" },
                        "data-draw-mode": if state_value.interaction_mode == InteractionMode::Draw { "true" } else { "false" },
                        if let Some(image_src) = selected_image_src {
                            div { class: "canvas-stage", "data-panzoom-stage": "true", "data-image-key": image_stage_key,
                                div { class: "canvas",
                                    img { class: "main-image", src: image_src, alt: "Selected dataset image" }
                                    if let Some(overlay_src) = overlay_data_uri {
                                        img { class: "overlay-image", src: overlay_src, alt: "Label overlay" }
                                    }
                                    if is_unlocked {
                                        if let Some(dims) = state_value.image_dimensions {
                                            CanvasOverlay {
                                                labels: visible_labels.clone(),
                                                image_width: dims.width,
                                                image_height: dims.height,
                                                interaction_mode: state_value.interaction_mode,
                                                drawing_vertices: state_value.drawing_vertices.clone(),
                                                onselect: move |label_idx: usize| {
                                                    let effects = reduce_state(app_state, AppAction::ToggleSelection { label_index: Some(label_idx) });
                                                    run_effects(api_base(), app_state, effects);
                                                },
                                                onaddvertex: move |point: Point| {
                                                    let effects = reduce_state(app_state, AppAction::AddDrawingVertex(point));
                                                    run_effects(api_base(), app_state, effects);
                                                },
                                                onfinishdrawing: move |_| {
                                                    let effects = reduce_state(app_state, AppAction::FinishDrawing);
                                                    run_effects(api_base(), app_state, effects);
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "empty-state",
                                h2 { "Viewer Shell Ready" }
                                p { "Expand the dataset tree on the left and select an image to load labels, metadata, and overlays from the Rust backend." }
                            }
                        }
                    }
                }

                div { class: "status-bar",
                    span { "Image: {image_name}" }
                    span { "Dimensions: {dimensions_text}" }
                    span { "Objects: {state_value.status.object_count}" }
                    span { "Mode: {mode_text}" }
                    if !state_value.filter_classes.is_empty() {
                        {
                            let filter_names = state_value.filter_classes.iter()
                                .map(|id| state_value.class_map.get(id)
                                    .cloned()
                                    .unwrap_or_else(|| format!("class {id}")))
                                .collect::<Vec<_>>()
                                .join(", ");
                            rsx! {
                                span {
                                    class: "status-pill filter-active-badge",
                                    "Filter: {filter_names}"
                                    button {
                                        style: "background:transparent;border:none;color:inherit;cursor:pointer;padding:0 0 0 6px;",
                                        onclick: move |_| {
                                            let effects = reduce_state(app_state, AppAction::ClearClassFilter);
                                            run_effects(api_base(), app_state, effects);
                                        },
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            aside { class: "panel right",
                div { class: "panel-inner",
                    div { class: "section-title", "Classes" }
                    div { class: "classes-scroll",
                        for class_id in class_ids.iter().copied() {
                            ClassLegendRow {
                                key: "legend-{class_id}",
                                class_id: class_id,
                                class_name: state_value.class_map.get(&class_id).cloned().unwrap_or_else(|| format!("class {class_id}")),
                                rgb: color_map().get(&class_id).copied().unwrap_or_else(|| yoda_core::default_color_for_class(class_id)),
                                hidden: state_value.hidden_classes.contains(&class_id),
                                ontoggle: move |cid: u32| {
                                    let currently_hidden = app_state().hidden_classes.contains(&cid);
                                    let effects = reduce_state(app_state, AppAction::SetClassVisibility {
                                        class_id: cid,
                                        visible: currently_hidden,
                                    });
                                    run_effects(api_base(), app_state, effects);
                                }
                            }
                        }
                    }
                    div { class: "section-title", "Objects" }
                    div { class: "objects-scroll",
                        for label in state_value.current_labels.iter().cloned() {
                            ObjectRow {
                                key: "object-{label.index}",
                                label: label.clone(),
                                class_name: state_value.class_map.get(&label.class_id).cloned().unwrap_or_else(|| format!("class {}", label.class_id)),
                                rgb: color_map().get(&label.class_id).copied().unwrap_or_else(|| yoda_core::default_color_for_class(label.class_id)),
                                is_selected: state_value.selected_object_index == Some(label.index),
                                is_hidden_by_class: state_value.hidden_classes.contains(&label.class_id),
                                lock_state: state_value.access_mode,
                                class_options: class_options.clone(),
                                onselect: move |index: usize| {
                                    let effects = reduce_state(app_state, AppAction::ToggleSelection { label_index: Some(index) });
                                    run_effects(api_base(), app_state, effects);
                                },
                                ontoggle_visibility: move |index: usize| {
                                    let visible = app_state()
                                        .current_labels
                                        .iter()
                                        .find(|label| label.index == index)
                                        .map(|label| !label.visible)
                                        .unwrap_or(true);
                                    let effects = reduce_state(app_state, AppAction::SetObjectVisibility { label_index: index, visible });
                                    run_effects(api_base(), app_state, effects);
                                },
                                onchange_class: move |payload: (usize, u32)| {
                                    let effects = reduce_state(app_state, AppAction::ChangeLabelClass {
                                        label_index: payload.0,
                                        class_id: payload.1,
                                    });
                                    run_effects(api_base(), app_state, effects);
                                },
                                ondelete: move |index: usize| {
                                    let effects = reduce_state(app_state, AppAction::DeleteLabel { label_index: index });
                                    run_effects(api_base(), app_state, effects);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ClassFilterBar(
    class_map: HashMap<u32, String>,
    color_map: HashMap<u32, (u8, u8, u8)>,
    filter_classes: BTreeSet<u32>,
    filter_mode: FilterMode,
    ontoggle_class: EventHandler<u32>,
    onset_mode: EventHandler<FilterMode>,
    onclear: EventHandler<()>,
) -> Element {
    if class_map.is_empty() {
        return rsx! { Fragment {} };
    }

    let is_active = !filter_classes.is_empty();
    let mut sorted_class_ids: Vec<u32> = class_map.keys().copied().collect();
    sorted_class_ids.sort_unstable();

    rsx! {
        div { class: "filter-bar",
            div { class: "filter-bar-header",
                span { class: "filter-label", "Filter" }
                if is_active {
                    button {
                        class: "filter-clear",
                        onclick: move |_| onclear.call(()),
                        "× Clear"
                    }
                }
            }
            div { class: "filter-mode",
                label {
                    input {
                        r#type: "radio",
                        name: "filter-mode",
                        checked: filter_mode == FilterMode::Any,
                        onchange: move |_| onset_mode.call(FilterMode::Any),
                    }
                    " Any"
                }
                label {
                    input {
                        r#type: "radio",
                        name: "filter-mode",
                        checked: filter_mode == FilterMode::All,
                        onchange: move |_| onset_mode.call(FilterMode::All),
                    }
                    " All"
                }
            }
            div { class: "filter-chips",
                for class_id in sorted_class_ids.into_iter() {
                    {
                        let name = class_map.get(&class_id).cloned().unwrap_or_else(|| format!("class {class_id}"));
                        let is_selected = filter_classes.contains(&class_id);
                        let rgb = color_map.get(&class_id).copied().unwrap_or_else(|| yoda_core::default_color_for_class(class_id));
                        let color_str = format!("rgb({},{},{})", rgb.0, rgb.1, rgb.2);
                        rsx! {
                            if is_selected {
                                button {
                                    key: "chip-{class_id}",
                                    class: "filter-chip active",
                                    style: "border-color: {color_str}; background: {color_str}22;",
                                    onclick: move |_| ontoggle_class.call(class_id),
                                    div { class: "chip-swatch", style: "background: {color_str}" }
                                    span { "{name}" }
                                }
                            } else {
                                button {
                                    key: "chip-{class_id}",
                                    class: "filter-chip",
                                    onclick: move |_| ontoggle_class.call(class_id),
                                    div { class: "chip-swatch", style: "background: {color_str}" }
                                    span { "{name}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FlatNodeView(
    row: VisibleRow,
    selected_image_path: Option<String>,
    ontoggle: EventHandler<u32>,
    onselect: EventHandler<String>,
) -> Element {
    let is_selected = row.kind == NodeKind::Image
        && selected_image_path.as_deref() == Some(row.path.as_str());
    let row_class = if is_selected { "tree-row selected" } else { "tree-row" };
    let indent_px = row.depth * 16;
    let arrow = match row.kind {
        NodeKind::Folder => {
            if row.has_children {
                if row.is_expanded { "▾" } else { "▸" }
            } else {
                "·"
            }
        }
        NodeKind::Image => " ",
    };
    let icon = match row.kind {
        NodeKind::Folder => NodeIcon::Folder,
        NodeKind::Image => NodeIcon::Image,
    };
    let node_id = row.id;
    let path = row.path.clone();
    let is_folder = row.kind == NodeKind::Folder;

    rsx! {
        div {
            class: "tree-node",
            style: "--indent: {indent_px}px;",
            button {
                class: row_class,
                onclick: move |_| {
                    if is_folder {
                        ontoggle.call(node_id);
                    } else {
                        onselect.call(path.clone());
                    }
                },
                span { class: "tree-arrow", "{arrow}" }
                TreeNodeIcon { icon }
                span { class: "tree-label", "{row.name}" }
            }
        }
    }
}

#[component]
fn ClassLegendRow(
    class_id: u32,
    class_name: String,
    rgb: (u8, u8, u8),
    hidden: bool,
    ontoggle: EventHandler<u32>,
) -> Element {
    let color = format!("rgb({},{},{})", rgb.0, rgb.1, rgb.2);
    rsx! {
        div { class: "legend-item",
            div { class: "swatch", style: "background: {color};" }
            div { class: "legend-name", "{class_name}" }
            button { onclick: move |_| ontoggle.call(class_id), if hidden { "Show" } else { "Hide" } }
        }
    }
}

#[component]
fn ObjectRow(
    label: LabelObject,
    class_name: String,
    rgb: (u8, u8, u8),
    is_selected: bool,
    is_hidden_by_class: bool,
    lock_state: AccessMode,
    class_options: Vec<(u32, String)>,
    onselect: EventHandler<usize>,
    ontoggle_visibility: EventHandler<usize>,
    onchange_class: EventHandler<(usize, u32)>,
    ondelete: EventHandler<usize>,
) -> Element {
    let color = format!("rgb({},{},{})", rgb.0, rgb.1, rgb.2);
    let row_class = if is_selected { "object-row selected" } else { "object-row" };
    let object_kind = match label.label_type {
        yoda_core::LabelType::Bbox => "bbox",
        yoda_core::LabelType::Polygon => "polygon",
    };

    rsx! {
        div {
            class: row_class,
            onclick: move |_| onselect.call(label.index),
            div { class: "swatch", style: "background: {color};" }
            button {
                class: "ghost",
                onclick: move |event| {
                    event.stop_propagation();
                    ontoggle_visibility.call(label.index);
                },
                if label.visible && !is_hidden_by_class { "Visible" } else { "Hidden" }
            }
            div { class: "object-name", "#{label.index + 1} {class_name} ({object_kind})" }
            select {
                value: "{label.class_id}",
                disabled: lock_state == AccessMode::Locked,
                onclick: move |event| event.stop_propagation(),
                onchange: move |event| {
                    if let Ok(class_id) = event.value().parse::<u32>() {
                        onchange_class.call((label.index, class_id));
                    }
                },
                for (class_id, option_label) in class_options.iter().cloned() {
                    option {
                        value: "{class_id}",
                        "{option_label}"
                    }
                }
            }
            button {
                class: "delete",
                disabled: lock_state == AccessMode::Locked,
                onclick: move |event| {
                    event.stop_propagation();
                    ondelete.call(label.index);
                },
                "Delete"
            }
        }
    }
}

/// Combined SVG overlay for both edit mode (dblclick-to-select hit areas) and
/// draw mode (click to place polygon vertices, click first vertex to close).
#[component]
fn CanvasOverlay(
    labels: Vec<LabelObject>,
    image_width: u32,
    image_height: u32,
    interaction_mode: InteractionMode,
    drawing_vertices: Vec<Point>,
    onselect: EventHandler<usize>,
    onaddvertex: EventHandler<Point>,
    onfinishdrawing: EventHandler<()>,
) -> Element {
    let mut cursor: Signal<Option<(f32, f32)>> = use_signal(|| None);
    let is_draw = interaction_mode == InteractionMode::Draw;
    let n = drawing_vertices.len();
    let can_close = n >= 3;

    // Precompute strings & data needed inside RSX --------------------------
    let committed_pts: String = drawing_vertices
        .iter()
        .map(|p| format!("{},{}", p.x, p.y))
        .collect::<Vec<_>>()
        .join(" ");

    let cursor_val = cursor();
    let cursor_near_close = can_close
        && cursor_val.is_some_and(|(cx, cy)| {
            let f = &drawing_vertices[0];
            let dx = cx - f.x;
            let dy = cy - f.y;
            (dx * dx + dy * dy).sqrt() < CLOSE_ZONE_RADIUS * 2.0
        });

    // (index, x, y) for rendering vertex markers
    let vertex_items: Vec<(usize, f32, f32)> =
        drawing_vertices.iter().enumerate().map(|(i, v)| (i, v.x, v.y)).collect();

    // Preview line: last vertex → cursor
    let preview_line: Option<(f32, f32, f32, f32)> = cursor_val
        .and_then(|(cx, cy)| drawing_vertices.last().map(|l| (l.x, l.y, cx, cy)));

    // Extra closing-edge preview: cursor → vertex[0] (shown when near close zone)
    let close_preview: Option<(f32, f32, f32, f32)> = if cursor_near_close {
        let first = drawing_vertices.first().map(|v| (v.x, v.y));
        cursor_val.zip(first).map(|((cx, cy), (fx, fy))| (cx, cy, fx, fy))
    } else {
        None
    };

    // Clone vertices for use inside the onclick closure
    let dv_click = drawing_vertices.clone();

    rsx! {
        svg {
            style: "position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none;",
            "viewBox": "0 0 {image_width} {image_height}",
            "preserveAspectRatio": "xMinYMin meet",

            // ── Edit mode: transparent hit areas so dblclick selects labels ──
            if !is_draw {
                for label in labels.into_iter().filter(|l| l.visible) {
                    HitAreaShape { key: "{label.index}", label, onselect }
                }
            }

            // ── Draw mode ────────────────────────────────────────────────────
            if is_draw {
                // Full-canvas transparent rect to capture mouse events.
                rect {
                    x: "0",
                    y: "0",
                    width: "{image_width}",
                    height: "{image_height}",
                    fill: "transparent",
                    "pointer-events": "all",
                    style: "cursor: crosshair;",
                    onmousemove: move |event: Event<MouseData>| {
                        let c = event.element_coordinates();
                        cursor.set(Some((c.x as f32, c.y as f32)));
                    },
                    onmouseleave: move |_| cursor.set(None),
                    onclick: move |event: Event<MouseData>| {
                        event.stop_propagation();
                        let c = event.element_coordinates();
                        let cx = c.x as f32;
                        let cy = c.y as f32;
                        // Close polygon if click lands near the first vertex.
                        if can_close {
                            let f = &dv_click[0];
                            let dx = cx - f.x;
                            let dy = cy - f.y;
                            if (dx * dx + dy * dy).sqrt() < CLOSE_ZONE_RADIUS * 2.0 {
                                onfinishdrawing.call(());
                                return;
                            }
                        }
                        onaddvertex.call(Point { x: cx, y: cy });
                    },
                }

                // Committed polygon fill + dashed border
                if n >= 2 {
                    polygon {
                        points: "{committed_pts}",
                        fill: "rgba(127,159,75,0.12)",
                        stroke: "#9fbe65",
                        "stroke-width": "1.5",
                        "stroke-dasharray": "8 4",
                        "pointer-events": "none",
                    }
                }

                // Preview line from last vertex to cursor
                if let Some((x1, y1, x2, y2)) = preview_line {
                    line {
                        x1: "{x1}", y1: "{y1}", x2: "{x2}", y2: "{y2}",
                        stroke: "#9fbe65",
                        "stroke-width": "1.5",
                        "stroke-dasharray": "5 3",
                        opacity: "0.7",
                        "pointer-events": "none",
                    }
                }

                // Closing-edge preview (cursor → vertex[0]) when hovering near close zone
                if let Some((x1, y1, x2, y2)) = close_preview {
                    line {
                        x1: "{x1}", y1: "{y1}", x2: "{x2}", y2: "{y2}",
                        stroke: "#c5e89e",
                        "stroke-width": "2",
                        "pointer-events": "none",
                    }
                }

                // Vertex markers
                for (i, vx, vy) in vertex_items {
                    if i == 0 && can_close {
                        // First vertex: close-zone indicator ring + dot
                        circle {
                            cx: "{vx}", cy: "{vy}",
                            r: "{CLOSE_ZONE_RADIUS}",
                            fill: if cursor_near_close { "rgba(127,159,75,0.35)" } else { "rgba(127,159,75,0.12)" },
                            stroke: "#9fbe65",
                            "stroke-width": "1.5",
                            "pointer-events": "none",
                        }
                        circle {
                            cx: "{vx}", cy: "{vy}",
                            r: "5",
                            fill: "#9fbe65",
                            stroke: "white",
                            "stroke-width": "1.5",
                            "pointer-events": "none",
                        }
                    } else {
                        circle {
                            cx: "{vx}", cy: "{vy}",
                            r: "5",
                            fill: "white",
                            stroke: "#9fbe65",
                            "stroke-width": "1.5",
                            "pointer-events": "none",
                        }
                    }
                }
            }
        }
    }
}

/// A single transparent hit-area shape (polygon or bbox rect) for one label.
#[component]
fn HitAreaShape(label: LabelObject, onselect: EventHandler<usize>) -> Element {
    let label_idx = label.index;
    match label.label_type {
        LabelType::Polygon => {
            let pts = label
                .pixel_points
                .iter()
                .map(|p| format!("{},{}", p.x, p.y))
                .collect::<Vec<_>>()
                .join(" ");
            rsx! {
                polygon {
                    points: "{pts}",
                    fill: "transparent",
                    stroke: "none",
                    "pointer-events": "all",
                    style: "cursor: pointer;",
                    ondoubleclick: move |event: Event<MouseData>| {
                        event.stop_propagation();
                        onselect.call(label_idx);
                    },
                }
            }
        }
        LabelType::Bbox => {
            let bbox = label.pixel_bbox;
            rsx! {
                rect {
                    x: "{bbox.x}",
                    y: "{bbox.y}",
                    width: "{bbox.width}",
                    height: "{bbox.height}",
                    fill: "transparent",
                    stroke: "none",
                    "pointer-events": "all",
                    style: "cursor: pointer;",
                    ondoubleclick: move |event: Event<MouseData>| {
                        event.stop_propagation();
                        onselect.call(label_idx);
                    },
                }
            }
        }
    }
}

fn reduce_state(mut app_state: Signal<AppState>, action: AppAction) -> Vec<AppEffect> {
    let mut state = app_state.write();
    match apply_action(&mut state, action) {
        Ok(result) => result.effects,
        Err(error) => {
            state.status.error_text = Some(error.to_string());
            Vec::new()
        }
    }
}

fn run_effects(api_base: String, app_state: Signal<AppState>, effects: Vec<AppEffect>) {
    for effect in effects {
        match effect {
            AppEffect::PersistLabels { image_path, labels } => {
                let mut app_state = app_state;
                let api_base = api_base.clone();
                spawn(async move {
                    match save_labels(&api_base, &image_path.to_string_lossy(), labels)
                        .await
                    {
                        Ok(_) => {
                            let mut state = app_state.write();
                            state.status.status_text =
                                Some(String::from("Labels saved"));
                            state.status.error_text = None;
                        }
                        Err(error) => set_status_error(app_state, error),
                    }
                });
            }
        }
    }
}

fn set_status_error(mut app_state: Signal<AppState>, message: String) {
    let mut state = app_state.write();
    state.status.error_text = Some(message);
}

fn sorted_class_ids(
    class_map: &HashMap<u32, String>,
    labels: &[LabelObject],
) -> Vec<u32> {
    let mut ids = class_map.keys().copied().collect::<BTreeSet<_>>();
    ids.extend(labels.iter().map(|label| label.class_id));
    ids.into_iter().collect()
}

fn render_overlay_data_uri(
    state: &AppState,
    color_map: &HashMap<u32, (u8, u8, u8)>,
    visible_labels: &[LabelObject],
) -> Option<String> {
    let dimensions = state.image_dimensions?;
    let svg = render_labels_to_svg(
        visible_labels,
        Some(color_map),
        Some(&state.class_map),
        &RenderOptions {
            show_bbox: state.show_bbox,
            show_segmask: state.show_segmask,
            show_class_id: state.show_class_id,
            show_class_name: state.show_class_name,
            selected_index: state.selected_object_index,
            image_width: dimensions.width,
        },
    );

    let wrapped = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"xMinYMin meet\">{svg}</svg>",
        dimensions.width, dimensions.height, dimensions.width, dimensions.height
    );

    Some(format!("data:image/svg+xml;utf8,{}", urlencoding::encode(&wrapped)))
}

fn build_image_url(api_base: &str, image_path: &str) -> String {
    build_endpoint(
        api_base,
        &format!("/api/image?image_path={}", urlencoding::encode(image_path)),
    )
}

fn build_endpoint(api_base: &str, path: &str) -> String {
    if api_base.is_empty() {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(origin) = window.location().origin() {
                    return format!("{origin}{path}");
                }
            }
        }

        path.to_string()
    } else {
        format!("{api_base}{path}")
    }
}

async fn fetch_labels(
    api_base: &str,
    image_path: &str,
) -> Result<LabelsResponse, String> {
    let endpoint =
        format!("/api/labels?image_path={}", urlencoding::encode(image_path));
    fetch_json(api_base, &endpoint).await
}

async fn fetch_flat_index(api_base: &str) -> Result<Vec<FlatNode>, String> {
    Ok(fetch_json::<FlatIndexResponse>(api_base, "/api/tree/flat").await?.nodes)
}

async fn fetch_class_map(api_base: &str) -> Result<HashMap<u32, String>, String> {
    Ok(fetch_json::<ClassMapResponse>(api_base, "/api/class-map").await?.class_map)
}

async fn fetch_color_map(api_base: &str) -> Result<HashMap<u32, (u8, u8, u8)>, String> {
    Ok(fetch_json::<ColorMapResponse>(api_base, "/api/color-map")
        .await?
        .color_map
        .into_iter()
        .map(|(class_id, rgb)| (class_id, (rgb[0], rgb[1], rgb[2])))
        .collect())
}

async fn fetch_class_index(
    api_base: &str,
) -> Result<HashMap<String, Vec<u32>>, String> {
    Ok(fetch_json::<ClassIndexResponse>(api_base, "/api/class-index").await?.entries)
}

async fn save_labels(
    api_base: &str,
    image_path: &str,
    labels: Vec<LabelObject>,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let endpoint = build_endpoint(
        api_base,
        &format!("/api/labels?image_path={}", urlencoding::encode(image_path)),
    );
    let response = client
        .put(endpoint)
        .json(&SaveLabelsRequest { labels })
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if response.status().is_success() {
        Ok(())
    } else {
        Err(response.text().await.unwrap_or_else(|_| String::from("label save failed")))
    }
}

async fn fetch_json<T>(api_base: &str, path: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let endpoint = build_endpoint(api_base, path);
    let response = reqwest::get(endpoint).await.map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(response
            .text()
            .await
            .unwrap_or_else(|_| String::from("request failed")));
    }

    response.json::<T>().await.map_err(|error| error.to_string())
}

#[component]
fn TreeNodeIcon(icon: NodeIcon) -> Element {
    match icon {
        NodeIcon::Folder => rsx! {
            svg {
                class: "tree-icon",
                "viewBox": "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.8",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                path { d: "M3 7.5a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" }
            }
        },
        NodeIcon::Image => rsx! {
            svg {
                class: "tree-icon",
                "viewBox": "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.8",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                rect { x: "4", y: "5", width: "16", height: "14", rx: "2" }
                circle { cx: "9", cy: "10", r: "1.5" }
                path { d: "m20 16-4.5-4.5L8 19" }
            }
        },
        NodeIcon::Placeholder => rsx! {
            svg {
                class: "tree-icon",
                "viewBox": "0 0 24 24",
                fill: "currentColor",
                circle { cx: "6", cy: "12", r: "1.3" }
                circle { cx: "12", cy: "12", r: "1.3" }
                circle { cx: "18", cy: "12", r: "1.3" }
            }
        },
    }
}
