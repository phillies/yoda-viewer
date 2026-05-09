use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;

use dioxus::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use yoda_app::{apply_action, AccessMode, AppAction, AppEffect, AppState, LoadedImage};
use yoda_core::{render_labels_to_svg, LabelObject, RenderOptions};
use yoda_data::{NodeIcon, TreeNode, LAZY_PLACEHOLDER_SUFFIX};

const APP_CSS: &str = r#"
:root {
    color-scheme: dark;
    --bg: #11161c;
    --panel: #19222b;
    --panel-2: #202c38;
    --panel-3: #293847;
    --line: #344557;
    --text: #ecf3f8;
    --muted: #94a6b8;
    --accent: #e25d3f;
    --accent-soft: rgba(226, 93, 63, 0.15);
    --good: #4dc48a;
    --warn: #ffb454;
}
* { box-sizing: border-box; }
html, body { margin: 0; height: 100%; background: radial-gradient(circle at top, #1a2430, #0d1217 58%); color: var(--text); font-family: "Segoe UI", "Inter", sans-serif; }
body { overflow: hidden; }
button, select { font: inherit; }
.app-shell { display: grid; grid-template-columns: 280px minmax(0, 1fr) 320px; height: 100vh; gap: 0; }
.panel { border-right: 1px solid var(--line); background: rgba(25, 34, 43, 0.88); backdrop-filter: blur(14px); min-height: 0; }
.panel.right { border-right: none; border-left: 1px solid var(--line); }
.panel-inner { display: flex; flex-direction: column; height: 100%; min-height: 0; }
.section-title { padding: 16px 18px 10px; font-size: 11px; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); }
.tree-scroll, .side-scroll { overflow: auto; min-height: 0; padding: 0 10px 14px; }
.tree-node { margin-left: var(--indent); }
.tree-row { width: 100%; display: flex; align-items: center; gap: 8px; border: 1px solid transparent; background: transparent; color: inherit; padding: 8px 10px; border-radius: 10px; text-align: left; cursor: pointer; }
.tree-row:hover { background: rgba(255,255,255,0.04); border-color: rgba(255,255,255,0.04); }
.tree-row.selected { background: var(--accent-soft); border-color: rgba(226, 93, 63, 0.35); }
.tree-arrow { width: 14px; text-align: center; color: var(--muted); }
.tree-icon { width: 18px; color: var(--muted); }
.tree-label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.content { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
.toolbar { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; padding: 14px 16px; border-bottom: 1px solid var(--line); background: rgba(17, 22, 28, 0.75); backdrop-filter: blur(10px); }
.toolbar-group { display: inline-flex; gap: 8px; align-items: center; padding: 6px; border-radius: 12px; background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.05); }
.toolbar button, .toolbar select { background: var(--panel-2); color: var(--text); border: 1px solid var(--line); border-radius: 10px; padding: 8px 12px; }
.toolbar button.active { background: var(--accent); border-color: var(--accent); }
.toolbar button:disabled, .toolbar select:disabled { opacity: 0.45; cursor: not-allowed; }
.status-pill { padding: 8px 12px; border-radius: 999px; font-size: 12px; border: 1px solid var(--line); background: rgba(255,255,255,0.03); }
.status-pill.locked { color: var(--warn); }
.status-pill.unlocked { color: var(--good); }
.viewport-wrap { flex: 1; min-height: 0; padding: 18px; overflow: auto; }
.viewport { min-height: 100%; border: 1px solid var(--line); border-radius: 22px; background: linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01)); display: flex; align-items: center; justify-content: center; padding: 18px; position: relative; }
.canvas { position: relative; display: inline-block; line-height: 0; max-width: 100%; }
.canvas img.main-image { display: block; max-width: min(100%, 1100px); max-height: calc(100vh - 220px); border-radius: 18px; box-shadow: 0 30px 80px rgba(0,0,0,0.45); }
.canvas img.overlay-image { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
.empty-state { max-width: 420px; text-align: center; color: var(--muted); }
.empty-state h2 { margin: 0 0 8px; color: var(--text); font-size: 28px; }
.empty-state p { margin: 0; line-height: 1.5; }
.status-bar { display: flex; flex-wrap: wrap; gap: 12px; padding: 12px 16px; border-top: 1px solid var(--line); color: var(--muted); background: rgba(17, 22, 28, 0.75); font-size: 12px; }
.legend-item, .object-row { display: flex; align-items: center; gap: 10px; padding: 10px 12px; margin-bottom: 8px; border-radius: 14px; background: rgba(255,255,255,0.03); border: 1px solid transparent; }
.object-row.selected { border-color: rgba(226, 93, 63, 0.35); background: var(--accent-soft); }
.swatch { width: 12px; height: 12px; border-radius: 999px; flex: none; }
.legend-name, .object-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.legend-item button, .object-row button, .object-row select { background: var(--panel-2); color: var(--text); border: 1px solid var(--line); border-radius: 10px; padding: 7px 10px; }
.object-row button.ghost { background: transparent; }
.object-row button.delete { color: #ff8e8e; }
.stack-gap { height: 10px; }
.message { margin: 8px 16px 0; padding: 10px 12px; border-radius: 12px; font-size: 13px; }
.message.error { background: rgba(255, 88, 88, 0.12); border: 1px solid rgba(255, 88, 88, 0.2); color: #ff9f9f; }
.message.info { background: rgba(77, 196, 138, 0.12); border: 1px solid rgba(77, 196, 138, 0.2); color: #9fe4bf; }
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
"#;

#[derive(Debug, Clone, Deserialize)]
struct TreeNodesResponse {
    nodes: Vec<TreeNode>,
}

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

#[derive(Debug, Clone, Serialize)]
struct SaveLabelsRequest {
    labels: Vec<LabelObject>,
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
    let tree_nodes = use_signal(Vec::<TreeNode>::new);
    let expanded_dirs = use_signal(BTreeSet::<String>::new);
    let color_map = use_signal(HashMap::<u32, (u8, u8, u8)>::new);
    let mut selected_image_path = use_signal(|| None::<String>);
    let tree_loading = use_signal(|| true);
    let image_loading = use_signal(|| false);

    {
        let api_base = api_base;
        let mut tree_nodes = tree_nodes;
        let mut tree_loading = tree_loading;
        let mut color_map = color_map;
        let app_state = app_state;
        use_effect(move || {
            let api_base_value = api_base();
            spawn(async move {
                tree_loading.set(true);
                match fetch_tree_root(&api_base_value).await {
                    Ok(nodes) => tree_nodes.set(nodes),
                    Err(error) => set_status_error(app_state, error),
                }

                match fetch_class_map(&api_base_value).await {
                    Ok(class_map) => {
                        let effects = reduce_state(app_state, AppAction::ClassMapLoaded(class_map));
                        run_effects(api_base_value.clone(), app_state, effects);
                    }
                    Err(error) => set_status_error(app_state, error),
                }

                match fetch_color_map(&api_base_value).await {
                    Ok(map) => color_map.set(map),
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
                        let effects = reduce_state(app_state, AppAction::ImageLoaded(loaded));
                        run_effects(api_base_value.clone(), app_state, effects);
                    }
                    Err(error) => set_status_error(app_state, error),
                }
                image_loading.set(false);
            });
        });
    }

    let state_value = app_state();
    let visible_labels = state_value.visible_labels();
    let overlay_data_uri = render_overlay_data_uri(&state_value, &color_map(), &visible_labels);
    let selected_image_src = state_value
        .current_image_path
        .as_ref()
        .map(|path| build_image_url(&api_base(), &path.to_string_lossy()));
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
    let status_class = if state_value.is_locked() { "status-pill locked" } else { "status-pill unlocked" };
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
        main { class: "app-shell",
            aside { class: "panel",
                div { class: "panel-inner",
                    div { class: "section-title", "Dataset" }
                    if tree_loading() {
                        div { class: "message info", "Loading dataset tree..." }
                    }
                    div { class: "tree-scroll",
                        for node in tree_nodes().iter().cloned() {
                            TreeNodeView {
                                key: "{node.id}",
                                node: node,
                                level: 0,
                                expanded_dirs: expanded_dirs(),
                                selected_image_path: selected_image_path_value.clone(),
                                ontoggle: move |node_id: String| {
                                    let api_base = api_base;
                                    let mut expanded_dirs = expanded_dirs;
                                    let mut tree_nodes = tree_nodes;
                                    let app_state = app_state;
                                    spawn(async move {
                                        let api_base_value = api_base();
                                        let is_expanded = expanded_dirs().contains(&node_id);
                                        if is_expanded {
                                            expanded_dirs.write().remove(&node_id);
                                            return;
                                        }

                                        expanded_dirs.write().insert(node_id.clone());
                                        if node_needs_children(&tree_nodes(), &node_id) {
                                            match fetch_tree_children(&api_base_value, &node_id).await {
                                                Ok(children) => {
                                                    tree_nodes.with_mut(|nodes| {
                                                        replace_children(nodes, &node_id, children);
                                                    });
                                                }
                                                Err(error) => set_status_error(app_state, error),
                                            }
                                        }
                                    });
                                },
                                onselect: move |image_path: String| {
                                    selected_image_path.set(Some(image_path));
                                }
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
                                let next_mode = if state_value.is_locked() { AccessMode::Unlocked } else { AccessMode::Locked };
                                let effects = reduce_state(app_state, AppAction::SetAccessMode(next_mode));
                                run_effects(api_base(), app_state, effects);
                            },
                            if state_value.is_locked() { "Unlock Editing" } else { "Lock Editing" }
                        }
                        span { class: status_class, if state_value.is_locked() { "Locked" } else { "Unlocked" } }
                    }
                    if image_loading() {
                        span { class: "status-pill", "Loading image..." }
                    }
                }

                if let Some(error_text) = state_value.status.error_text.clone() {
                    div { class: "message error", "{error_text}" }
                }
                if let Some(status_text) = state_value.status.status_text.clone() {
                    div { class: "message info", "{status_text}" }
                }

                div { class: "viewport-wrap",
                    div { class: "viewport",
                        if let Some(image_src) = selected_image_src {
                            div { class: "canvas",
                                img { class: "main-image", src: image_src, alt: "Selected dataset image" }
                                if let Some(overlay_src) = overlay_data_uri {
                                    img { class: "overlay-image", src: overlay_src, alt: "Label overlay" }
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
                }
            }

            aside { class: "panel right",
                div { class: "panel-inner",
                    div { class: "section-title", "Classes" }
                    div { class: "side-scroll",
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
                        div { class: "stack-gap" }
                        div { class: "section-title", "Objects" }
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
fn TreeNodeView(
    node: TreeNode,
    level: usize,
    expanded_dirs: BTreeSet<String>,
    selected_image_path: Option<String>,
    ontoggle: EventHandler<String>,
    onselect: EventHandler<String>,
) -> Element {
    let is_folder = node.icon == NodeIcon::Folder;
    let is_expanded = expanded_dirs.contains(&node.id);
    let is_selected = selected_image_path.as_deref() == Some(node.id.as_str());
    let children_to_render = if is_folder && is_expanded {
        node.children
            .into_iter()
            .filter(|child| child.icon != NodeIcon::Placeholder)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    rsx! {
        div {
            class: "tree-node",
            style: "--indent: {level * 16}px;",
            button {
                class: if is_selected { "tree-row selected" } else { "tree-row" },
                onclick: move |_| {
                    if is_folder {
                        ontoggle.call(node.id.clone());
                    } else {
                        onselect.call(node.id.clone());
                    }
                },
                span { class: "tree-arrow", if is_folder { if is_expanded { "v" } else { ">" } } else { "-" } }
                span { class: "tree-icon", if is_folder { "DIR" } else { "IMG" } }
                span { class: "tree-label", "{node.label}" }
            }
            for child in children_to_render {
                TreeNodeView {
                    key: "{child.id}",
                    node: child,
                    level: level + 1,
                    expanded_dirs: expanded_dirs.clone(),
                    selected_image_path: selected_image_path.clone(),
                    ontoggle: ontoggle,
                    onselect: onselect,
                }
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
                    match save_labels(&api_base, &image_path.to_string_lossy(), labels).await {
                        Ok(_) => {
                            let mut state = app_state.write();
                            state.status.status_text = Some(String::from("Labels saved"));
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

fn replace_children(nodes: &mut [TreeNode], node_id: &str, children: Vec<TreeNode>) -> bool {
    for node in nodes.iter_mut() {
        if node.id == node_id {
            node.children = children;
            return true;
        }
        if replace_children(&mut node.children, node_id, children.clone()) {
            return true;
        }
    }
    false
}

fn node_needs_children(nodes: &[TreeNode], node_id: &str) -> bool {
    nodes.iter().any(|node| {
        if node.id == node_id {
            return node.children.iter().any(|child| child.id.ends_with(LAZY_PLACEHOLDER_SUFFIX));
        }
        node_needs_children(&node.children, node_id)
    })
}

fn sorted_class_ids(class_map: &HashMap<u32, String>, labels: &[LabelObject]) -> Vec<u32> {
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
        },
    );

    let wrapped = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">{svg}</svg>",
        dimensions.width,
        dimensions.height,
        dimensions.width,
        dimensions.height
    );

    Some(format!(
        "data:image/svg+xml;utf8,{}",
        urlencoding::encode(&wrapped)
    ))
}

fn build_image_url(api_base: &str, image_path: &str) -> String {
    build_endpoint(api_base, &format!("/api/image?image_path={}", urlencoding::encode(image_path)))
}

fn build_endpoint(api_base: &str, path: &str) -> String {
    if api_base.is_empty() {
        path.to_string()
    } else {
        format!("{api_base}{path}")
    }
}

async fn fetch_tree_root(api_base: &str) -> Result<Vec<TreeNode>, String> {
    Ok(fetch_json::<TreeNodesResponse>(api_base, "/api/tree").await?.nodes)
}

async fn fetch_tree_children(api_base: &str, path: &str) -> Result<Vec<TreeNode>, String> {
    let endpoint = format!("/api/tree/children?path={}", urlencoding::encode(path));
    Ok(fetch_json::<TreeNodesResponse>(api_base, &endpoint).await?.nodes)
}

async fn fetch_labels(api_base: &str, image_path: &str) -> Result<LabelsResponse, String> {
    let endpoint = format!("/api/labels?image_path={}", urlencoding::encode(image_path));
    fetch_json(api_base, &endpoint).await
}

async fn fetch_class_map(api_base: &str) -> Result<HashMap<u32, String>, String> {
    Ok(fetch_json::<ClassMapResponse>(api_base, "/api/class-map")
        .await?
        .class_map)
}

async fn fetch_color_map(api_base: &str) -> Result<HashMap<u32, (u8, u8, u8)>, String> {
    Ok(fetch_json::<ColorMapResponse>(api_base, "/api/color-map")
        .await?
        .color_map
        .into_iter()
        .map(|(class_id, rgb)| (class_id, (rgb[0], rgb[1], rgb[2])))
        .collect())
}

async fn save_labels(api_base: &str, image_path: &str, labels: Vec<LabelObject>) -> Result<(), String> {
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
    let response = reqwest::get(endpoint)
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(response
            .text()
            .await
            .unwrap_or_else(|_| String::from("request failed")));
    }

    response.json::<T>().await.map_err(|error| error.to_string())
}