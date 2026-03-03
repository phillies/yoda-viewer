"""YoDa Browser — NiceGUI-based UI for viewing YOLO segmentation labels.

V4: UX improvements — consistent dark theme, compact toolbar, status bar,
improved inspector panel, keyboard shortcuts.
"""

from __future__ import annotations

from pathlib import Path

from loguru import logger
from nicegui import app, events, ui
from nicegui.elements.interactive_image import InteractiveImage
from PIL import Image

from yoda import fileops
from yoda.config import YoDaConfig
from yoda.dataset import load_class_map
from yoda.label import (
    LabelObject,
    create_label_from_pixels,
    delete_label,
    parse_yolo_labels,
    render_labels_to_svg,
    write_yolo_labels,
)

# ---------------------------------------------------------------------------
# Theme CSS  (V4)
# ---------------------------------------------------------------------------

_THEME_CSS = """\
:root {
    --bg-base: #1e1e2e;
    --bg-surface: #282840;
    --bg-surface-hover: #32325a;
    --bg-elevated: #363660;
    --text-primary: #e0e0f0;
    --text-secondary: #a0a0c0;
    --text-muted: #6a6a90;
    --accent: #7c8aff;
    --accent-hover: #9aa4ff;
    --danger: #ff6b6b;
    --success: #69db7c;
    --border: #3a3a60;
}
html, body {
    margin: 0; padding: 0; overflow: hidden;
    height: 100%;
    background: var(--bg-base);
    color: var(--text-primary);
}
.nicegui-content { padding: 0 !important; }

/* Smooth transitions */
button, .q-toggle, .q-checkbox, .q-select, .q-btn {
    transition: background-color 0.15s ease, color 0.15s ease,
                border-color 0.15s ease, opacity 0.15s ease !important;
}

/* Scrollbar */
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-track { background: var(--bg-base); }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: var(--text-muted); }

/* File tree */
.q-tree__node-header-content {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}
.q-tree__node--selected > .q-tree__node-header {
    background: var(--bg-elevated) !important;
    border-left: 3px solid var(--accent);
}
.q-tree__node-header:hover {
    background: var(--bg-surface-hover) !important;
}
.q-tree,
.q-tree__node-header-content,
.q-tree__node-header-content div { color: var(--text-primary) !important; }
.q-tree .q-icon { color: var(--text-secondary) !important; }

/* Tooltip */
.q-tooltip {
    font-size: 12px !important;
    background: var(--bg-elevated) !important;
    border: 1px solid var(--border) !important;
    color: var(--text-primary) !important;
}

/* Drawer */
.q-drawer {
    background: var(--bg-surface) !important;
    border-left: 1px solid var(--border) !important;
}

/* Splitter handle */
.q-splitter__separator {
    background: var(--border) !important;
}

/* Button group container */
.yoda-btn-group {
    display: inline-flex;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    gap: 0;
}
.yoda-btn-group .q-btn {
    border-radius: 0 !important;
    min-height: 28px !important;
    min-width: 28px !important;
    padding: 2px 7px !important;
}

/* Toggle button states */
.yoda-toggle-active {
    background: var(--accent) !important;
    color: white !important;
}
.yoda-toggle-active .q-icon { color: white !important; }
.yoda-toggle-inactive {
    background: transparent !important;
    color: var(--text-muted) !important;
}
.yoda-toggle-inactive:hover {
    background: var(--bg-surface-hover) !important;
    color: var(--text-primary) !important;
}

/* Status bar */
.yoda-status-bar {
    height: 24px;
    background: var(--bg-surface);
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 12px;
    font-family: 'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace;
    color: var(--text-secondary);
    gap: 16px;
    flex-shrink: 0;
}

/* Object row */
.yoda-object-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-radius: 4px;
    background: transparent;
    transition: background 0.15s ease;
    width: 100%;
    cursor: default;
}
.yoda-object-row:hover {
    background: var(--bg-surface-hover);
}
.yoda-object-row:hover .yoda-delete-btn {
    opacity: 1 !important;
}
.yoda-object-selected {
    background: var(--bg-elevated) !important;
    border-left: 3px solid var(--accent);
}

/* Section headers */
.yoda-section-header {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 1px;
    text-transform: uppercase;
    color: var(--text-secondary);
    padding: 8px 0 4px 0;
}

/* Checkerboard canvas background */
.yoda-canvas-bg {
    background-color: var(--bg-base);
    background-image:
        linear-gradient(45deg, #252540 25%, transparent 25%),
        linear-gradient(-45deg, #252540 25%, transparent 25%),
        linear-gradient(45deg, transparent 75%, #252540 75%),
        linear-gradient(-45deg, transparent 75%, #252540 75%);
    background-size: 20px 20px;
    background-position: 0 0, 0 10px, 10px -10px, -10px 0;
}

/* Marching ants animation for selected polygons */
@keyframes marchingAnts {
    to { stroke-dashoffset: -22; }
}

/* Select styling */
.q-field__control { color: var(--text-primary) !important; }
.q-field__native { color: var(--text-primary) !important; }

/* Dropdown / popup menu */
.q-menu {
    background: var(--bg-surface) !important;
    border: 1px solid var(--border) !important;
}
.q-menu .q-item {
    color: var(--text-primary) !important;
}
.q-menu .q-item:hover,
.q-menu .q-item--active {
    background: var(--bg-surface-hover) !important;
}
.q-menu .q-item__label {
    color: var(--text-primary) !important;
}
"""


def _find_and_update_node(
    nodes: list[dict[str, object]],
    node_id: str,
    children: list[dict[str, object]],
) -> bool:
    """Recursively find *node_id* in *nodes* and replace its children.

    Returns ``True`` if the node was found and updated, ``False`` otherwise.
    """
    for node in nodes:
        if node.get("id") == node_id:
            node["children"] = children
            return True
        sub = node.get("children")
        if isinstance(sub, list) and _find_and_update_node(sub, node_id, children):
            return True
    return False


class YoDaBrowser:
    """Main browser UI for YoDa — manages layout, controls and state."""

    # --- state ---
    config: YoDaConfig
    image_base_path: Path
    label_base_path: Path
    class_map: dict[int, str]
    tree_data: list[dict[str, object]]

    # display toggles
    show_bbox: bool = False
    show_segmask: bool = True
    show_class_id: bool = False
    show_class_name: bool = False

    # per-image state
    current_labels: list[LabelObject] = []
    current_label_path: Path | None = None
    current_image_path: Path | None = None
    image_object: Image.Image | None = None

    # V2: class visibility filter
    hidden_classes: set[int]

    # V3: interaction mode and drawing state
    interaction_mode: str  # "edit" or "draw"
    drawing_vertices: list[tuple[float, float]]
    drawing_class_id: int
    selected_object_index: int | None

    # UI elements (assigned during render)
    interactive_image: InteractiveImage
    image_wrapper: ui.element
    image_container: ui.element
    object_list_container: ui.column  # type: ignore[assignment]
    message_label: ui.label  # type: ignore[assignment]
    edit_mode_btn: ui.button  # type: ignore[assignment]
    draw_mode_btn: ui.button  # type: ignore[assignment]
    delete_btn: ui.button  # type: ignore[assignment]
    draw_class_select: ui.select  # type: ignore[assignment]
    file_tree: ui.tree  # type: ignore[assignment]

    # V4 toggle buttons
    seg_toggle_btn: ui.button  # type: ignore[assignment]
    bbox_toggle_btn: ui.button  # type: ignore[assignment]
    classid_toggle_btn: ui.button  # type: ignore[assignment]
    classname_toggle_btn: ui.button  # type: ignore[assignment]

    # V4 status bar
    status_file_label: ui.label  # type: ignore[assignment]
    status_dims_label: ui.label  # type: ignore[assignment]
    status_objects_label: ui.label  # type: ignore[assignment]
    status_mode_label: ui.label  # type: ignore[assignment]

    # V4 object count badge
    objects_count_badge: ui.badge  # type: ignore[assignment]

    def __init__(self, config: YoDaConfig) -> None:
        self.config = config
        self.image_base_path = config.settings.image_base_path.resolve()
        self.label_base_path = config.settings.label_base_path.resolve()
        self.class_map = load_class_map(config.settings.class_info)
        self.tree_data = fileops.get_file_tree(self.image_base_path)
        self.current_labels = []
        self.current_label_path = None
        self.current_image_path = None
        self.hidden_classes = set()
        self.interaction_mode = "edit"
        self.drawing_vertices = []
        self.drawing_class_id = next(iter(self.class_map), 0)
        self.selected_object_index = None
        # Lazy tree loading: track which directory nodes have been expanded/loaded
        self._expanded_nodes: set[str] = set()
        self._loaded_nodes: set[str] = set()

    # ------------------------------------------------------------------
    # Rendering
    # ------------------------------------------------------------------

    def render(self) -> None:
        """Build the main page layout."""
        ui.add_head_html(f"<style>{_THEME_CSS}</style>")

        # --- Right drawer (inspector) — open by default ---
        with (
            ui.right_drawer(value=True, fixed=False)
            .props("width=280 bordered")
            .style(
                "background: var(--bg-surface) !important; "
                "border-left: 1px solid var(--border) !important; "
                "padding: 8px 10px;"
            ) as self.right_drawer
        ):
            self._build_inspector_panel()

        # --- Main layout: left tree | center ---
        with (
            ui.splitter(value=15).classes("w-full").style("height: 100vh;") as splitter
        ):
            # Left pane: file tree
            with splitter.before:
                with (
                    ui.column()
                    .classes("w-full h-full")
                    .style(
                        "background: var(--bg-surface); "
                        "border-right: 1px solid var(--border); "
                        "padding: 8px;"
                    )
                ):
                    ui.label("IMAGES").classes("yoda-section-header")
                    with ui.scroll_area().classes("w-full").style("flex: 1;"):
                        self.file_tree = ui.tree(
                            self.tree_data,
                            on_select=self._on_tree_select,
                            on_expand=self._on_tree_expand,
                        )
                        self.file_tree.classes("w-full")
                        self.file_tree.props("dense")

            # Right pane: toolbar + image + status bar
            with splitter.after:
                with (
                    ui.column()
                    .classes("w-full h-full")
                    .style("overflow: hidden; gap: 0;")
                ):
                    self._build_toolbar()
                    self._build_image_viewer()
                    self._build_status_bar()

        # Restore the last-viewed image if one was saved for this user.
        ui.timer(0.1, self._restore_last_image, once=True)

    # ------------------------------------------------------------------
    # Inspector panel (right)
    # ------------------------------------------------------------------

    def _build_inspector_panel(self) -> None:
        """Build the right inspector panel content."""
        # --- Class legend ---
        ui.label("CLASSES").classes("yoda-section-header")
        with ui.scroll_area().classes("w-full").style("max-height: 45%;"):
            self.class_legend_container = (
                ui.column().classes("w-full").style("gap: 2px;")
            )
            self._build_class_legend()

        ui.element("div").style(
            "height: 1px; background: var(--border); margin: 8px 0;"
        )

        # --- Object list ---
        with ui.row().classes("items-center w-full").style("gap: 8px;"):
            ui.label("OBJECTS").classes("yoda-section-header").style("flex: 1;")
            self.objects_count_badge = ui.badge("0").props(
                "color=grey-8 text-color=white"
            )

        with ui.scroll_area().classes("w-full").style("flex: 1;"):
            self.object_list_container = (
                ui.column().classes("w-full").style("gap: 4px;")
            )
            with self.object_list_container:
                ui.label("No image loaded").style(
                    "font-size: 12px; color: var(--text-muted);"
                )

    # ------------------------------------------------------------------
    # Toolbar
    # ------------------------------------------------------------------

    def _build_toolbar(self) -> None:
        """Build the compact top toolbar with grouped controls."""
        with (
            ui.row()
            .classes("w-full items-center yoda-toolbar")
            .style(
                "background: var(--bg-surface); "
                "height: 40px; min-height: 40px; max-height: 40px; "
                "padding: 0 8px; gap: 8px; "
                "border-bottom: 1px solid var(--border); "
                "z-index: 10; flex-shrink: 0;"
            )
        ):
            # --- Display toggles ---
            with ui.element("div").classes("yoda-btn-group"):
                self.seg_toggle_btn = (
                    ui.button(icon="layers", on_click=self._toggle_segmask)
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-active")
                    .tooltip("Segmentation Masks (M)")
                )
                self.bbox_toggle_btn = (
                    ui.button(
                        icon="check_box_outline_blank", on_click=self._toggle_bbox
                    )
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-inactive")
                    .tooltip("Bounding Boxes (B)")
                )
                self.classid_toggle_btn = (
                    ui.button(icon="tag", on_click=self._toggle_class_id)
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-inactive")
                    .tooltip("Class ID (I)")
                )
                self.classname_toggle_btn = (
                    ui.button(icon="label", on_click=self._toggle_class_name)
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-inactive")
                    .tooltip("Class Name (N)")
                )

            # Separator
            ui.element("div").style(
                "width: 1px; height: 20px; background: var(--border);"
            )

            # --- Mode buttons ---
            with ui.element("div").classes("yoda-btn-group"):
                self.edit_mode_btn = (
                    ui.button(icon="pan_tool", on_click=self._set_edit_mode)
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-active")
                    .tooltip("Edit / Pan mode (E)")
                )
                self.draw_mode_btn = (
                    ui.button(icon="edit", on_click=self._set_draw_mode)
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-inactive")
                    .tooltip("Draw mode (D)")
                )
                self.delete_btn = (
                    ui.button(icon="delete_outline", on_click=self._on_delete_selected)
                    .props("flat dense unelevated size=sm")
                    .classes("yoda-toggle-inactive")
                    .tooltip("Delete selected (Del)")
                )
            self.delete_btn.set_enabled(False)

            # Class selector for draw mode
            class_options: dict[int, str] = {}
            for cid, cname in self.class_map.items():
                class_options[cid] = cname
            if not class_options:
                class_options[0] = "class 0"
            self.draw_class_select = (
                ui.select(
                    options=class_options,
                    value=self.drawing_class_id,
                    on_change=lambda e: setattr(self, "drawing_class_id", e.value),
                )
                .props("dense options-dense outlined")
                .style("min-width: 120px; font-size: 12px; color: var(--text-primary);")
                .tooltip("Class for new objects")
            )

            # Separator
            ui.element("div").style(
                "width: 1px; height: 20px; background: var(--border);"
            )

            # --- Zoom controls ---
            with ui.element("div").classes("yoda-btn-group"):
                ui.button(icon="fit_screen", on_click=self._fit_to_screen).props(
                    "flat dense unelevated size=sm"
                ).classes("yoda-toggle-inactive").tooltip("Fit to screen (F)")
                ui.button(icon="zoom_in", on_click=self._zoom_in).props(
                    "flat dense unelevated size=sm"
                ).classes("yoda-toggle-inactive").tooltip("Zoom in (+)")
                ui.button(icon="zoom_out", on_click=self._zoom_out).props(
                    "flat dense unelevated size=sm"
                ).classes("yoda-toggle-inactive").tooltip("Zoom out (-)")
                ui.button(icon="crop_free", on_click=self._zoom_100).props(
                    "flat dense unelevated size=sm"
                ).classes("yoda-toggle-inactive").tooltip("100% zoom")

            # Spacer to push inspector toggle to far right
            ui.element("div").style("flex: 1;")

            # Inspector drawer toggle
            ui.button(
                icon="view_sidebar",
                on_click=lambda: self.right_drawer.toggle(),
            ).props("flat dense unelevated round size=sm").style(
                "color: var(--text-secondary);"
            ).tooltip("Toggle inspector")

    # ------------------------------------------------------------------
    # Image viewer
    # ------------------------------------------------------------------

    def _build_image_viewer(self) -> None:
        """Build the image container with the interactive image."""
        self.image_container = (
            ui.element("div")
            .classes("w-full relative yoda-canvas-bg")
            .style("flex: 1 1 0; overflow: hidden; position: relative; cursor: grab;")
        )

        with self.image_container:
            self.message_label = ui.label("Select an image from the tree").style(
                "position: absolute; top: 50%; left: 50%; "
                "transform: translate(-50%, -50%); z-index: 1; "
                "color: var(--text-muted); font-size: 14px;"
            )

            self.image_wrapper = ui.element("div").style(
                "transform-origin: 0 0; "
                "transform: translate(0px, 0px) scale(1); "
                "display: inline-block; position: absolute; "
                "top: 0; left: 0;"
            )
            with self.image_wrapper:
                self.interactive_image = (
                    ui.interactive_image(
                        "",
                        cross=False,
                        on_mouse=self._on_image_click,
                        events=["click"],
                    )
                    .classes("w-auto h-auto")
                    .style("max-width: none; max-height: none; display: block;")
                )
            self.interactive_image.visible = False
            self.image_wrapper.visible = False

        ui.keyboard(on_key=self._on_key_event)
        self._inject_zoom_pan_js()

    # ------------------------------------------------------------------
    # Status bar
    # ------------------------------------------------------------------

    def _build_status_bar(self) -> None:
        """Build the bottom status bar."""
        with ui.element("div").classes("yoda-status-bar"):
            self.status_file_label = ui.label("No file loaded").style(
                "flex: 1; overflow: hidden; text-overflow: ellipsis; "
                "white-space: nowrap;"
            )
            self.status_dims_label = ui.label("—")
            self.status_objects_label = ui.label("0 objects")
            self.status_mode_label = ui.label("Edit mode").style(
                "color: var(--accent);"
            )

    def _update_status_bar(self) -> None:
        """Refresh all status bar fields."""
        # File name
        if self.current_image_path is not None:
            self.status_file_label.text = self.current_image_path.name
        else:
            self.status_file_label.text = "No file loaded"

        # Dimensions
        if self.image_object is not None:
            w, h = self.image_object.width, self.image_object.height
            self.status_dims_label.text = f"{w} × {h}"
        else:
            self.status_dims_label.text = "—"

        # Object count
        n = len(self.current_labels)
        self.status_objects_label.text = f"{n} object{'s' if n != 1 else ''}"

        # Mode
        if self.interaction_mode == "draw":
            self.status_mode_label.text = "Draw mode"
            self.status_mode_label.style("color: var(--success);")
        else:
            self.status_mode_label.text = "Edit mode"
            self.status_mode_label.style("color: var(--accent);")

    # ------------------------------------------------------------------
    # Display toggle handlers (V4 icon buttons)
    # ------------------------------------------------------------------

    def _toggle_segmask(self) -> None:
        self.show_segmask = not self.show_segmask
        self._update_toggle_btn(self.seg_toggle_btn, self.show_segmask)
        self._refresh_overlay()

    def _toggle_bbox(self) -> None:
        self.show_bbox = not self.show_bbox
        self._update_toggle_btn(self.bbox_toggle_btn, self.show_bbox)
        self._refresh_overlay()

    def _toggle_class_id(self) -> None:
        self.show_class_id = not self.show_class_id
        self._update_toggle_btn(self.classid_toggle_btn, self.show_class_id)
        self._refresh_overlay()

    def _toggle_class_name(self) -> None:
        self.show_class_name = not self.show_class_name
        self._update_toggle_btn(self.classname_toggle_btn, self.show_class_name)
        self._refresh_overlay()

    @staticmethod
    def _update_toggle_btn(btn: ui.button, active: bool) -> None:
        """Update a toggle button's visual state."""
        if active:
            btn.classes(remove="yoda-toggle-inactive", add="yoda-toggle-active")
        else:
            btn.classes(remove="yoda-toggle-active", add="yoda-toggle-inactive")

    # ------------------------------------------------------------------
    # Event handlers
    # ------------------------------------------------------------------

    def _on_tree_select(self, e: events.ValueChangeEventArguments) -> None:
        """Handle file selection from the tree."""
        if not e.value:
            return
        selected_path = Path(e.value)
        if selected_path.is_file():
            self._load_image(selected_path)

    def _on_tree_expand(self, e: events.ValueChangeEventArguments) -> None:
        """Lazily load directory children when a node is expanded for the first time."""
        expanded_ids: set[str] = set(e.value) if e.value else set()
        newly_expanded = expanded_ids - self._expanded_nodes
        self._expanded_nodes = expanded_ids

        for node_id in newly_expanded:
            if node_id not in self._loaded_nodes:
                self._lazy_load_node(node_id)

    def _lazy_load_node(self, node_id: str) -> None:
        """Replace the lazy placeholder children of a directory node."""
        path = Path(node_id)
        if not path.is_dir():
            return
        logger.info(f"Lazy-loading tree children for: {path}")
        children = fileops.get_dir_children(path)
        if _find_and_update_node(self.tree_data, node_id, children):
            self._loaded_nodes.add(node_id)
            # NiceGUI's ui.tree documents that node updates are pushed by
            # writing directly to _props['nodes'] followed by update().
            self.file_tree._props["nodes"] = self.tree_data  # noqa: SLF001
            self.file_tree.update()

    def _restore_last_image(self) -> None:
        """Reload the image that was open when the user last left the app."""
        last = app.storage.user.get("last_image")
        if not last:
            return
        path = Path(last)
        if path.exists() and path.is_file():
            logger.info(f"Restoring last image: {path}")
            try:
                self._load_image(path)
            except ValueError as exc:
                logger.warning(
                    f"Failed to restore last image {path!s}; clearing stored value: {exc}"
                )
                # Clear invalid last_image to avoid repeated restore failures
                app.storage.user["last_image"] = None

    # ------------------------------------------------------------------
    # V2: Class legend, visibility, class change
    # ------------------------------------------------------------------

    def _build_class_legend(self) -> None:
        """Populate the class legend with visibility checkboxes."""
        self.class_legend_container.clear()
        with self.class_legend_container:
            if not self.class_map:
                ui.label("No classes loaded").style(
                    "font-size: 12px; color: var(--text-muted);"
                )
            else:
                for class_id, name in self.class_map.items():
                    color_str = self.config.get_color_string(class_id)
                    visible = class_id not in self.hidden_classes
                    with (
                        ui.row()
                        .classes("items-center w-full")
                        .style(
                            "gap: 6px; padding: 2px 4px; border-radius: 3px; "
                            "min-height: 28px;"
                        )
                    ):
                        ui.checkbox(
                            value=visible,
                            on_change=lambda e, cid=class_id: (
                                self._on_class_visibility_change(cid, e.value)
                            ),
                        ).props("dense size=sm color=white")
                        # Color swatch — 14×14 rounded square
                        ui.element("div").style(
                            f"width: 14px; height: 14px; "
                            f"border-radius: 3px; "
                            f"background: {color_str}; flex-shrink: 0;"
                        )
                        ui.label(name).style(
                            "font-size: 13px; color: var(--text-primary);"
                        )

    def _on_class_visibility_change(self, class_id: int, visible: bool) -> None:
        """Toggle visibility for all objects of a given class."""
        if visible:
            self.hidden_classes.discard(class_id)
        else:
            self.hidden_classes.add(class_id)
        for label in self.current_labels:
            if label.class_id in self.hidden_classes:
                label.visible = False
            else:
                label.visible = True
        self._refresh_overlay()
        self._rebuild_object_list()

    def _on_object_visibility_toggle(self, label_index: int, visible: bool) -> None:
        """Toggle visibility of a single object."""
        for label in self.current_labels:
            if label.index == label_index:
                label.visible = visible
                break
        self._refresh_overlay()
        self._rebuild_object_list()

    def _on_class_change(self, label_index: int, new_class_id: int) -> None:
        """Change the class of a single object and save to disk."""
        for label in self.current_labels:
            if label.index == label_index:
                label.class_id = new_class_id
                label.visible = new_class_id not in self.hidden_classes
                break
        self._refresh_overlay()
        self._rebuild_object_list()
        self._save_labels()

    def _save_labels(self) -> None:
        """Persist the current labels back to disk."""
        if self.current_label_path is None:
            return
        write_yolo_labels(self.current_label_path, self.current_labels)

    # ------------------------------------------------------------------
    # V3: Interaction modes — edit / draw + delete
    # ------------------------------------------------------------------

    def _set_edit_mode(self) -> None:
        """Switch to edit / pan mode and discard any in-progress drawing."""
        self.interaction_mode = "edit"
        self.drawing_vertices = []
        self._update_mode_buttons()
        self._update_cursor_and_handlers()
        self._refresh_overlay()
        self._update_status_bar()

    def _set_draw_mode(self) -> None:
        """Switch to draw mode for adding new objects."""
        if self.image_object is None:
            ui.notify("Load an image first", type="warning")
            return
        self.interaction_mode = "draw"
        self.drawing_vertices = []
        self._update_mode_buttons()
        self._update_cursor_and_handlers()
        self._refresh_overlay()
        self._update_status_bar()

    def _update_mode_buttons(self) -> None:
        """Highlight the active mode button."""
        if self.interaction_mode == "edit":
            self.edit_mode_btn.classes(
                remove="yoda-toggle-inactive", add="yoda-toggle-active"
            )
            self.draw_mode_btn.classes(
                remove="yoda-toggle-active", add="yoda-toggle-inactive"
            )
        else:
            self.edit_mode_btn.classes(
                remove="yoda-toggle-active", add="yoda-toggle-inactive"
            )
            self.draw_mode_btn.classes(
                remove="yoda-toggle-inactive", add="yoda-toggle-active"
            )

    def _update_cursor_and_handlers(self) -> None:
        """Update the cursor style and JS handlers based on the mode."""
        cid = self.image_container.id
        if self.interaction_mode == "draw":
            ui.run_javascript(
                f"(function(){{ "
                f"  var c = document.getElementById('c{cid}');"
                f"  if (c) {{ c.style.cursor = 'crosshair'; c._yzDrawMode = true; }}"
                f"}})();"
            )
        else:
            ui.run_javascript(
                f"(function(){{ "
                f"  var c = document.getElementById('c{cid}');"
                f"  if (c) {{ c.style.cursor = 'grab'; c._yzDrawMode = false; }}"
                f"}})();"
            )

    @staticmethod
    def _point_in_polygon(
        x: float, y: float, points: list[tuple[float, float]]
    ) -> bool:
        """Ray-casting point-in-polygon test."""
        n = len(points)
        inside = False
        px, py = x, y
        j = n - 1
        for i in range(n):
            xi, yi = points[i]
            xj, yj = points[j]
            if ((yi > py) != (yj > py)) and (
                px < (xj - xi) * (py - yi) / (yj - yi) + xi
            ):
                inside = not inside
            j = i
        return inside

    def _on_image_click(self, e: events.MouseEventArguments) -> None:
        """Handle click on the interactive image."""
        if self.image_object is None:
            return

        x, y = e.image_x, e.image_y

        if self.interaction_mode == "edit":
            hit_index: int | None = None
            for label_obj in reversed(self.current_labels):
                if not label_obj.visible:
                    continue
                if label_obj.label_type == "polygon" and self._point_in_polygon(
                    x, y, label_obj.pixel_points
                ):
                    hit_index = label_obj.index
                    break
                if label_obj.label_type == "bbox":
                    bx, by, bw, bh = label_obj.pixel_bbox
                    if bx <= x <= bx + bw and by <= y <= by + bh:
                        hit_index = label_obj.index
                        break
            if hit_index == self.selected_object_index:
                self.selected_object_index = None
            else:
                self.selected_object_index = hit_index
            self._refresh_overlay()
            self._rebuild_object_list()
            self._update_delete_button()
            return

        if self.interaction_mode != "draw":
            return
        logger.info(f"Draw vertex at ({x:.1f}, {y:.1f})")

        if len(self.drawing_vertices) >= 3:
            fx, fy = self.drawing_vertices[0]
            dist = ((x - fx) ** 2 + (y - fy) ** 2) ** 0.5
            if dist < 10:
                self._finish_drawing()
                return

        self.drawing_vertices.append((x, y))
        self._refresh_overlay()

    def _on_key_event(self, e: events.KeyEventArguments) -> None:
        """Handle keyboard events — shortcuts + draw mode keys."""
        if not e.action.keydown:
            return

        key = e.key

        # --- Draw mode keys ---
        if key == "Escape":
            if self.interaction_mode == "draw" and self.drawing_vertices:
                self.drawing_vertices = []
                self._refresh_overlay()
                ui.notify("Drawing discarded", type="info")
            elif self.interaction_mode == "draw":
                self._set_edit_mode()
            return
        if key == "Enter":
            if self.interaction_mode == "draw" and len(self.drawing_vertices) >= 3:
                self._finish_drawing()
            elif self.interaction_mode == "draw":
                ui.notify("Need at least 3 vertices", type="warning")
            return

        # --- Global keyboard shortcuts (V4) ---
        if key.key == "e" if hasattr(key, "key") else key == "e":
            self._set_edit_mode()
        elif key.key == "d" if hasattr(key, "key") else key == "d":
            self._set_draw_mode()
        elif key == "Delete":
            if self.selected_object_index is not None:
                self._on_delete_selected()
        elif key.key == "f" if hasattr(key, "key") else key == "f":
            self._fit_to_screen()
        elif key.key == "m" if hasattr(key, "key") else key == "m":
            self._toggle_segmask()
        elif key.key == "b" if hasattr(key, "key") else key == "b":
            self._toggle_bbox()
        elif key.key == "i" if hasattr(key, "key") else key == "i":
            self._toggle_class_id()
        elif key.key == "n" if hasattr(key, "key") else key == "n":
            self._toggle_class_name()

    def _finish_drawing(self) -> None:
        """Complete the polygon drawing and store the new object."""
        if self.image_object is None or len(self.drawing_vertices) < 3:
            return

        new_index = len(self.current_labels)
        new_label = create_label_from_pixels(
            self.drawing_vertices,
            self.image_object.width,
            self.image_object.height,
            self.drawing_class_id,
            new_index,
        )
        new_label.visible = new_label.class_id not in self.hidden_classes

        self.current_labels.append(new_label)
        self.drawing_vertices = []
        self._save_labels()
        self._refresh_overlay()
        self._rebuild_object_list()
        self._update_status_bar()
        ui.notify(f"Object #{new_index + 1} added", type="positive")

    def _on_delete_object(self, label_index: int) -> None:
        """Delete an object by index, save and refresh."""
        logger.info(f"Deleting object at index {label_index}")
        try:
            self.current_labels = delete_label(self.current_labels, label_index)
            if self.selected_object_index == label_index:
                self.selected_object_index = None
            elif self.selected_object_index is not None:
                valid_indices = {lbl.index for lbl in self.current_labels}
                if self.selected_object_index not in valid_indices:
                    self.selected_object_index = None
            self._save_labels()
            self._refresh_overlay()
            ui.notify("Object deleted", type="info")
            self._rebuild_object_list()
            self._update_status_bar()
            logger.info(
                f"Delete complete. {len(self.current_labels)} objects remaining"
            )
        except Exception:
            logger.exception("Error in _on_delete_object")
        finally:
            self._update_delete_button()

    def _on_delete_selected(self) -> None:
        """Delete the currently selected object (toolbar button handler)."""
        if self.selected_object_index is not None:
            self._on_delete_object(self.selected_object_index)

    def _update_delete_button(self) -> None:
        """Enable or disable the toolbar delete button based on selection."""
        self.delete_btn.set_enabled(self.selected_object_index is not None)

    def _render_drawing_preview_svg(self) -> str:
        """Render an SVG preview of the polygon being drawn."""
        if not self.drawing_vertices:
            return ""

        color = self.config.get_color_string(self.drawing_class_id)
        parts: list[str] = []

        if len(self.drawing_vertices) >= 2:
            points_str = " ".join(f"{p[0]},{p[1]}" for p in self.drawing_vertices)
            parts.append(
                f'<polyline points="{points_str}" '
                f'fill="none" stroke="{color}" '
                f'stroke-width="2" stroke-dasharray="5,3" />'
            )

        for i, (vx, vy) in enumerate(self.drawing_vertices):
            r = 5 if i == 0 else 3
            fill = "white" if i == 0 else color
            parts.append(
                f'<circle cx="{vx}" cy="{vy}" r="{r}" '
                f'fill="{fill}" stroke="{color}" stroke-width="2" />'
            )

        if len(self.drawing_vertices) >= 3:
            lx, ly = self.drawing_vertices[-1]
            fx, fy = self.drawing_vertices[0]
            parts.append(
                f'<line x1="{lx}" y1="{ly}" x2="{fx}" y2="{fy}" '
                f'stroke="{color}" stroke-width="1" '
                f'stroke-dasharray="3,3" opacity="0.5" />'
            )

        return "".join(parts)

    # ------------------------------------------------------------------
    # Zoom / Pan (client-side JS)
    # ------------------------------------------------------------------

    def _inject_zoom_pan_js(self) -> None:
        """Inject client-side JavaScript for wheel-zoom and drag-pan."""
        cid = self.image_container.id
        wid = self.image_wrapper.id
        js = (
            "(function init() {"
            f"  var c = document.getElementById('c{cid}');"
            f"  var w = document.getElementById('c{wid}');"
            "  if (!c || !w) { setTimeout(init, 100); return; }"
            "  var s = { scale:1, panX:0, panY:0,"
            "            dragging:false, sx:0, sy:0, spx:0, spy:0 };"
            "  c._yz = s;"
            "  c._yzDrawMode = false;"
            "  function ap() {"
            "    w.style.transform = "
            "      'translate(' + s.panX + 'px,' + s.panY + 'px) "
            "       scale(' + s.scale + ')';"
            "  }"
            "  c.addEventListener('wheel', function(e) {"
            "    e.preventDefault();"
            "    var r = c.getBoundingClientRect();"
            "    var cx = e.clientX - r.left;"
            "    var cy = e.clientY - r.top;"
            "    var f = e.deltaY > 0 ? 0.9 : 1.1;"
            "    var ns = Math.max(0.01, Math.min(s.scale * f, 50));"
            "    var ratio = ns / s.scale;"
            "    s.panX = cx - (cx - s.panX) * ratio;"
            "    s.panY = cy - (cy - s.panY) * ratio;"
            "    s.scale = ns;"
            "    ap();"
            "  }, {passive:false});"
            "  c.addEventListener('mousedown', function(e) {"
            "    if (e.button !== 0 || c._yzDrawMode) return;"
            "    s.dragging = true;"
            "    s.sx = e.clientX; s.sy = e.clientY;"
            "    s.spx = s.panX; s.spy = s.panY;"
            "    c.style.cursor = 'grabbing';"
            "    e.preventDefault();"
            "  });"
            "  window.addEventListener('mousemove', function(e) {"
            "    if (!s.dragging) return;"
            "    s.panX = s.spx + (e.clientX - s.sx);"
            "    s.panY = s.spy + (e.clientY - s.sy);"
            "    ap();"
            "  });"
            "  window.addEventListener('mouseup', function() {"
            "    if (s.dragging) {"
            "      s.dragging = false;"
            "      if (!c._yzDrawMode) c.style.cursor = 'grab';"
            "    }"
            "  });"
            "  ap();"
            "})();"
        )
        ui.add_body_html(f"<script>{js}</script>")

    def _run_zoom_js(self, body: str) -> None:
        """Run a JS snippet with access to container *c*, wrapper *w*, state *s*."""
        cid = self.image_container.id
        wid = self.image_wrapper.id
        ui.run_javascript(
            f"(function(){{ "
            f"  var c = document.getElementById('c{cid}');"
            f"  var w = document.getElementById('c{wid}');"
            f"  if (!c || !w || !c._yz) return;"
            f"  var s = c._yz;"
            f"  function ap(){{ "
            f"    w.style.transform = "
            f"      'translate(' + s.panX + 'px,' + s.panY + 'px) "
            f"       scale(' + s.scale + ')'; "
            f"  }} "
            f"  {body} "
            f"}})();"
        )

    def _fit_to_screen(self) -> None:
        """Scale the image so it fits entirely in the visible container."""
        if self.image_object is None:
            return
        iw = self.image_object.width
        ih = self.image_object.height
        self._run_zoom_js(
            f"setTimeout(function(){{ "
            f"  var cw = c.clientWidth, ch = c.clientHeight; "
            f"  s.scale = Math.min(cw / {iw}, ch / {ih}); "
            f"  s.panX = (cw - {iw} * s.scale) / 2; "
            f"  s.panY = (ch - {ih} * s.scale) / 2; "
            f"  ap(); "
            f"}}, 150);"
        )

    def _zoom_100(self) -> None:
        """Reset zoom to 100 %."""
        if self.image_object is None:
            return
        iw = self.image_object.width
        ih = self.image_object.height
        self._run_zoom_js(
            f"var cw = c.clientWidth, ch = c.clientHeight; "
            f"s.scale = 1; "
            f"s.panX = (cw - {iw}) / 2; "
            f"s.panY = (ch - {ih}) / 2; "
            f"ap();"
        )

    def _zoom_in(self) -> None:
        """Zoom in by 25 %, centred on the viewport."""
        self._zoom_by_factor(1.25)

    def _zoom_out(self) -> None:
        """Zoom out by 20 %, centred on the viewport."""
        self._zoom_by_factor(0.8)

    def _zoom_by_factor(self, factor: float) -> None:
        """Apply a zoom *factor* centred on the viewport centre."""
        if self.image_object is None:
            return
        self._run_zoom_js(
            f"var cx = c.clientWidth / 2, cy = c.clientHeight / 2; "
            f"var ns = Math.max(0.01, Math.min(s.scale * {factor}, 50)); "
            f"var r = ns / s.scale; "
            f"s.panX = cx - (cx - s.panX) * r; "
            f"s.panY = cy - (cy - s.panY) * r; "
            f"s.scale = ns; "
            f"ap();"
        )

    # ------------------------------------------------------------------
    # Image loading and overlay
    # ------------------------------------------------------------------

    def _load_image(self, image_path: Path) -> None:
        """Load an image and its labels, update the display."""
        logger.info(f"Loading image: {image_path}")
        self.message_label.visible = False
        self.interactive_image.visible = True
        self.image_wrapper.visible = True

        self.selected_object_index = None
        self._update_delete_button()

        self.image_object = Image.open(image_path)
        self.current_image_path = image_path
        self.interactive_image.source = self.image_object

        # Persist a path relative to the dataset root so it can be restored
        # robustly after a browser refresh, even if the root moves.
        try:
            relative_image_path = image_path.relative_to(self.image_base_path)
        except ValueError:
            # If the image is not under image_base_path, fall back to the filename.
            logger.warning(
                "Image %s is not under image_base_path %s; "
                "falling back to storing only the filename.",
                image_path,
                self.image_base_path,
            )
            relative_image_path = image_path.name
        app.storage.user["last_image"] = str(relative_image_path)

        self._fit_to_screen()

        label_path = self.label_base_path / image_path.relative_to(
            self.image_base_path
        ).with_suffix(".txt")
        self.current_label_path = label_path
        logger.info(f"Label file: {label_path}")

        self.current_labels = parse_yolo_labels(
            label_path, self.image_object.width, self.image_object.height
        )

        for label in self.current_labels:
            label.visible = label.class_id not in self.hidden_classes

        self._refresh_overlay()
        self._rebuild_object_list()
        self._update_status_bar()

    def _refresh_overlay(self) -> None:
        """Re-render the SVG overlay from cached labels with current toggles."""
        if self.image_object is None:
            return

        svg = render_labels_to_svg(
            self.current_labels,
            color_map=self.config.color_map_tuples,
            show_bbox=self.show_bbox,
            show_segmask=self.show_segmask,
            show_class_id=self.show_class_id,
            show_class_name=self.show_class_name,
            class_map=self.class_map,
            selected_index=self.selected_object_index,
        )
        svg += self._render_drawing_preview_svg()
        self.interactive_image.content = svg

    def _rebuild_object_list(self) -> None:
        """Populate the inspector object list with V4 styled rows."""
        self.object_list_container.clear()

        # Update badge count
        n = len(self.current_labels)
        self.objects_count_badge.text = str(n)

        if not self.current_labels:
            with self.object_list_container:
                msg = (
                    "No labels found"
                    if self.current_label_path is None
                    or not self.current_label_path.exists()
                    else "No objects in label file"
                )
                ui.label(msg).style("font-size: 12px; color: var(--text-muted);")
            return

        class_options: dict[int, str] = {}
        for cid, cname in self.class_map.items():
            class_options[cid] = cname
        for label_obj in self.current_labels:
            if label_obj.class_id not in class_options:
                class_options[label_obj.class_id] = f"class {label_obj.class_id}"

        with self.object_list_container:
            for label_obj in self.current_labels:
                color_str = self.config.get_color_string(label_obj.class_id)
                idx = label_obj.index
                is_selected = idx == self.selected_object_index

                row_classes = "yoda-object-row"
                if is_selected:
                    row_classes += " yoda-object-selected"

                with ui.element("div").classes(row_classes):
                    # Eye icon toggle
                    ui.button(
                        icon=("visibility" if label_obj.visible else "visibility_off"),
                        on_click=lambda _e, i=idx, v=label_obj.visible: (
                            self._on_object_visibility_toggle(i, not v)
                        ),
                    ).props("flat dense size=xs padding=none").style(
                        "color: var(--text-secondary); min-width: 24px;"
                    )

                    # Color dot
                    ui.element("div").style(
                        f"width: 10px; height: 10px; border-radius: 50%; "
                        f"background: {color_str}; flex-shrink: 0;"
                    )

                    # Object index
                    ui.label(f"#{idx + 1}").style(
                        "font-size: 12px; font-weight: 600; "
                        "color: var(--text-secondary); min-width: 24px;"
                    )

                    # Class dropdown
                    ui.select(
                        options=class_options,
                        value=label_obj.class_id,
                        on_change=lambda e, i=idx: self._on_class_change(i, e.value),
                    ).props("dense options-dense borderless").style(
                        "min-width: 80px; flex: 1; font-size: 12px; "
                        "color: var(--text-primary);"
                    )

                    # Delete button (hidden until hover via CSS)
                    ui.button(
                        icon="delete",
                        on_click=lambda _e, i=idx: self._on_delete_object(i),
                    ).props("flat dense size=xs padding=none").classes(
                        "yoda-delete-btn"
                    ).style(
                        "color: var(--danger); opacity: 0; min-width: 24px;"
                    ).tooltip("Delete object")
