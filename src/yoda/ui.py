"""YoDa Browser — NiceGUI-based UI for viewing YOLO segmentation labels."""

from __future__ import annotations

from pathlib import Path

from loguru import logger
from nicegui import events, ui
from nicegui.elements.interactive_image import InteractiveImage
from PIL import Image

from yoda import fileops
from yoda.config import YoDaConfig
from yoda.dataset import load_class_map
from yoda.label import (
    LabelObject,
    parse_yolo_labels,
    render_labels_to_svg,
    write_yolo_labels,
)


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
    image_object: Image.Image | None = None

    # V2: class visibility filter
    hidden_classes: set[int]

    # UI elements (assigned during render)
    interactive_image: InteractiveImage
    image_wrapper: ui.element  # inner wrapper that gets transform
    image_container: ui.element  # outer scrollable container
    object_list_container: ui.column  # type: ignore[assignment]
    message_label: ui.label  # type: ignore[assignment]

    def __init__(self, config: YoDaConfig) -> None:
        self.config = config
        self.image_base_path = config.settings.image_base_path.resolve()
        self.label_base_path = config.settings.label_base_path.resolve()
        self.class_map = load_class_map(config.settings.class_info)
        self.tree_data = fileops.get_file_tree(self.image_base_path)
        self.current_labels = []
        self.current_label_path = None
        self.hidden_classes = set()

    # ------------------------------------------------------------------
    # Rendering
    # ------------------------------------------------------------------

    def render(self) -> None:
        """Build the main page layout."""
        # Remove default page margin / padding so the app fills the viewport
        ui.add_head_html(
            "<style>"
            "html, body { margin: 0; padding: 0; overflow: hidden; "
            "height: 100%; }"
            ".nicegui-content { padding: 0 !important; }"
            "</style>"
        )

        # --- Right drawer: object list ---
        with (
            ui.right_drawer(value=False, fixed=False)
            .classes("bg-gray-800 p-3")
            .props("width=300 bordered") as self.right_drawer
        ):
            # --- Class legend with visibility toggles ---
            ui.label("Classes").classes("text-lg font-bold text-white mb-2")
            with ui.scroll_area().classes("w-full").style("max-height: 50%;"):
                self.class_legend_container = ui.column().classes("w-full gap-1")
                self._build_class_legend()

            ui.separator().classes("my-2")

            # --- Object list ---
            ui.label("Objects").classes("text-lg font-bold text-white mb-2")
            with ui.scroll_area().classes("w-full").style("max-height: 50%;"):
                self.object_list_container = ui.column().classes("w-full gap-1")
                with self.object_list_container:
                    ui.label("No image loaded").classes("text-xs text-gray-400")

        # --- Main layout: left tree | center image ---
        with (
            ui.splitter(value=20).classes("w-full").style("height: 100vh;") as splitter
        ):
            # Left pane: file tree
            with splitter.before, ui.column().classes("w-full h-full p-2"):
                ui.label("Images").classes("text-lg font-bold")
                tree = ui.tree(self.tree_data, on_select=self._on_tree_select)
                tree.classes("w-full")

            # Right pane (center): toolbar + image viewer
            with (
                splitter.after,
                ui.column()
                .classes("w-full h-full relative bg-gray-900")
                .style("overflow: hidden;"),
            ):
                self._build_toolbar()
                self._build_image_viewer()

    def _build_toolbar(self) -> None:
        """Build the top toolbar with display toggles and class legend."""
        with ui.row().classes(
            "w-full p-2 bg-gray-800 items-center gap-4 z-10 flex-wrap"
        ):
            ui.label("Display:").classes("text-white font-bold")

            ui.switch(
                "Seg. Masks",
                value=self.show_segmask,
                on_change=self._on_toggle_segmask,
            ).props("color=green dense")

            ui.switch(
                "Bounding Boxes",
                value=self.show_bbox,
                on_change=self._on_toggle_bbox,
            ).props("color=blue dense")

            ui.switch(
                "Class ID",
                value=self.show_class_id,
                on_change=self._on_toggle_class_id,
            ).props("color=orange dense")

            ui.switch(
                "Class Name",
                value=self.show_class_name,
                on_change=self._on_toggle_class_name,
            ).props("color=purple dense")

            ui.separator().props("vertical").classes("mx-2")

            # Zoom controls
            ui.button(
                icon="fit_screen",
                on_click=self._fit_to_screen,
            ).props("flat color=white dense").tooltip("Fit to screen")
            ui.button(
                icon="zoom_in",
                on_click=self._zoom_in,
            ).props("flat color=white dense").tooltip("Zoom in")
            ui.button(
                icon="zoom_out",
                on_click=self._zoom_out,
            ).props("flat color=white dense").tooltip("Zoom out")
            ui.button(
                "100%",
                on_click=self._zoom_100,
            ).props("flat color=white dense").tooltip("Zoom to 100%")

            ui.separator().props("vertical").classes("mx-2")

            # Toggle button for object list drawer
            ui.button(
                icon="list",
                on_click=lambda: self.right_drawer.toggle(),
            ).props("flat color=white dense").tooltip("Toggle object list")

    def _build_image_viewer(self) -> None:
        """Build the image container with the interactive image."""
        # Outer container: fills remaining space, hidden overflow (pan via JS)
        self.image_container = (
            ui.element("div")
            .classes("w-full relative")
            .style("flex: 1 1 0; overflow: hidden; position: relative; cursor: grab;")
        )

        with self.image_container:
            self.message_label = (
                ui.label("Select an image from the tree")
                .classes("text-gray-400")
                .style(
                    "position: absolute; top: 50%; left: 50%; "
                    "transform: translate(-50%, -50%); z-index: 1;"
                )
            )

            # Inner wrapper that receives the CSS transform for zoom + pan
            self.image_wrapper = ui.element("div").style(
                "transform-origin: 0 0; "
                "transform: translate(0px, 0px) scale(1); "
                "display: inline-block; position: absolute; "
                "top: 0; left: 0;"
            )
            with self.image_wrapper:
                self.interactive_image = (
                    ui.interactive_image("", cross=False)
                    .classes("w-auto h-auto")
                    .style("max-width: none; max-height: none; display: block;")
                )
            self.interactive_image.visible = False
            self.image_wrapper.visible = False

        # Inject client-side JS for wheel-zoom and drag-pan
        self._inject_zoom_pan_js()

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

    def _on_toggle_segmask(self, e: events.ValueChangeEventArguments) -> None:
        self.show_segmask = e.value
        self._refresh_overlay()

    def _on_toggle_bbox(self, e: events.ValueChangeEventArguments) -> None:
        self.show_bbox = e.value
        self._refresh_overlay()

    def _on_toggle_class_id(self, e: events.ValueChangeEventArguments) -> None:
        self.show_class_id = e.value
        self._refresh_overlay()

    def _on_toggle_class_name(self, e: events.ValueChangeEventArguments) -> None:
        self.show_class_name = e.value
        self._refresh_overlay()

    # ------------------------------------------------------------------
    # V2: Class legend, class visibility, object hide/show, class change
    # ------------------------------------------------------------------

    def _build_class_legend(self) -> None:
        """Populate the class legend with visibility checkboxes."""
        self.class_legend_container.clear()
        with self.class_legend_container:
            if not self.class_map:
                ui.label("No classes loaded").classes("text-xs text-gray-400")
            else:
                for class_id, name in self.class_map.items():
                    color_str = self.config.get_color_string(class_id)
                    visible = class_id not in self.hidden_classes
                    with ui.row().classes("items-center gap-2 w-full"):
                        ui.checkbox(
                            value=visible,
                            on_change=lambda e, cid=class_id: (
                                self._on_class_visibility_change(cid, e.value)
                            ),
                        ).props("dense size=xs color=white")
                        ui.element("div").style(
                            f"width: 10px; height: 10px; "
                            f"border-radius: 50%; "
                            f"background: {color_str}; flex-shrink: 0;"
                        )
                        ui.label(name).classes("text-xs text-white")

    def _on_class_visibility_change(
        self, class_id: int, visible: bool
    ) -> None:
        """Toggle visibility for all objects of a given class."""
        if visible:
            self.hidden_classes.discard(class_id)
        else:
            self.hidden_classes.add(class_id)
        # Update the visible flag on each label
        for label in self.current_labels:
            if label.class_id in self.hidden_classes:
                label.visible = False
            else:
                label.visible = True
        self._refresh_overlay()
        self._rebuild_object_list()

    def _on_object_visibility_toggle(
        self, label_index: int, visible: bool
    ) -> None:
        """Toggle visibility of a single object."""
        for label in self.current_labels:
            if label.index == label_index:
                label.visible = visible
                break
        self._refresh_overlay()
        self._rebuild_object_list()

    def _on_class_change(
        self, label_index: int, new_class_id: int
    ) -> None:
        """Change the class of a single object and save to disk."""
        for label in self.current_labels:
            if label.index == label_index:
                label.class_id = new_class_id
                # Update visibility based on hidden_classes
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
            "    if (e.button !== 0) return;"
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
            "      c.style.cursor = 'grab';"
            "    }"
            "  });"
            "  ap();"
            "})();"
        )
        ui.add_body_html(f"<script>{js}</script>")

    def _run_zoom_js(self, body: str) -> None:
        """Run a JS snippet that has access to container *c*, wrapper *w*,
        and the zoom-state object *s*.  The snippet must set s.scale,
        s.panX, s.panY then call ap()."""
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
        """Scale the image so it fits entirely inside the visible container."""
        if self.image_object is None:
            return
        iw = self.image_object.width
        ih = self.image_object.height
        # A short timeout lets NiceGUI's WebSocket DOM patch land first.
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
        """Reset zoom to 100 % (1 pixel = 1 pixel)."""
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

        # Open image
        self.image_object = Image.open(image_path)
        self.interactive_image.source = self.image_object

        # Auto-fit the new image into the container
        self._fit_to_screen()

        # Resolve corresponding label file
        label_path = self.label_base_path / image_path.relative_to(
            self.image_base_path
        ).with_suffix(".txt")
        self.current_label_path = label_path
        logger.info(f"Label file: {label_path}")

        # Parse labels
        self.current_labels = parse_yolo_labels(
            label_path, self.image_object.width, self.image_object.height
        )

        # Apply class visibility from the hidden_classes set
        for label in self.current_labels:
            label.visible = label.class_id not in self.hidden_classes

        # Render overlay
        self._refresh_overlay()

        # Update object list in drawer
        self._rebuild_object_list()

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
        )
        self.interactive_image.content = svg

    def _rebuild_object_list(self) -> None:
        """Populate the right-drawer object list with V2 controls.

        Each object row has:
        - An eye icon to toggle visibility (hide/show)
        - A color dot
        - A class dropdown to change the object's class
        - The object type badge
        """
        self.object_list_container.clear()

        if not self.current_labels:
            with self.object_list_container:
                msg = (
                    "No labels found"
                    if self.current_label_path is None
                    or not self.current_label_path.exists()
                    else "No objects in label file"
                )
                ui.label(msg).classes("text-xs text-gray-400")
            return

        # Build class options for the dropdown
        class_options: dict[int, str] = {}
        for cid, cname in self.class_map.items():
            class_options[cid] = cname
        # Also add any class IDs present in labels but not in class_map
        for label_obj in self.current_labels:
            if label_obj.class_id not in class_options:
                class_options[label_obj.class_id] = f"class {label_obj.class_id}"

        with self.object_list_container:
            for label_obj in self.current_labels:
                color_str = self.config.get_color_string(label_obj.class_id)
                obj_type = "bbox" if label_obj.label_type == "bbox" else "poly"
                idx = label_obj.index

                with ui.row().classes("items-center gap-1 w-full"):
                    # Eye icon toggle for visibility
                    ui.button(
                        icon=(
                            "visibility" if label_obj.visible else "visibility_off"
                        ),
                        on_click=lambda _e, i=idx, v=label_obj.visible: (
                            self._on_object_visibility_toggle(i, not v)
                        ),
                    ).props("flat dense size=xs color=white padding=none")

                    # Color dot
                    ui.element("div").style(
                        f"width: 10px; height: 10px; border-radius: 50%; "
                        f"background: {color_str}; flex-shrink: 0;"
                    )

                    # Object index
                    ui.label(f"#{idx + 1}").classes("text-white text-xs")

                    # Class dropdown
                    ui.select(
                        options=class_options,
                        value=label_obj.class_id,
                        on_change=lambda e, i=idx: self._on_class_change(
                            i, e.value
                        ),
                    ).props(
                        "dense options-dense borderless"
                    ).classes("text-xs text-white").style(
                        "min-width: 80px; flex: 1;"
                    )

                    # Type badge
                    ui.label(f"[{obj_type}]").classes(
                        "text-gray-400 text-xs ml-auto"
                    )
