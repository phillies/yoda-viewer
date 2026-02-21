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
from yoda.label import LabelObject, parse_yolo_labels, render_labels_to_svg


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
    image_object: Image.Image | None = None

    # UI elements (assigned during render)
    interactive_image: InteractiveImage
    object_list_container: ui.column  # type: ignore[assignment]
    message_label: ui.label  # type: ignore[assignment]

    def __init__(self, config: YoDaConfig) -> None:
        self.config = config
        self.image_base_path = config.settings.image_base_path.resolve()
        self.label_base_path = config.settings.label_base_path.resolve()
        self.class_map = load_class_map(config.settings.class_info)
        self.tree_data = fileops.get_file_tree(self.image_base_path)
        self.current_labels = []

    # ------------------------------------------------------------------
    # Rendering
    # ------------------------------------------------------------------

    def render(self) -> None:
        """Build the main page layout."""
        # --- Right drawer: object list ---
        with (
            ui.right_drawer(value=False, fixed=False)
            .classes("bg-gray-800 p-3")
            .props("width=280 bordered") as self.right_drawer
        ):
            # --- Class legend ---
            ui.label("Classes").classes("text-lg font-bold text-white mb-2")
            with ui.scroll_area().classes("w-full").style("max-height: 50%;"):
                if not self.class_map:
                    ui.label("No classes loaded").classes("text-xs text-gray-400")
                else:
                    with ui.column().classes("w-full gap-1"):
                        for class_id, name in self.class_map.items():
                            color_str = self.config.get_color_string(class_id)
                            with ui.row().classes("items-center gap-2"):
                                ui.element("div").style(
                                    f"width: 10px; height: 10px; "
                                    f"border-radius: 50%; "
                                    f"background: {color_str}; flex-shrink: 0;"
                                )
                                ui.label(name).classes("text-xs text-white")

            ui.separator().classes("my-2")

            # --- Object list ---
            ui.label("Objects").classes("text-lg font-bold text-white mb-2")
            with ui.scroll_area().classes("w-full").style("max-height: 50%;"):
                self.object_list_container = ui.column().classes("w-full gap-1")
                with self.object_list_container:
                    ui.label("No image loaded").classes("text-xs text-gray-400")

        # --- Main layout: left tree | center image ---
        with ui.splitter(value=20).classes("w-full h-screen") as splitter:
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

            # Toggle button for object list drawer
            ui.button(
                icon="list",
                on_click=lambda: self.right_drawer.toggle(),
            ).props("flat color=white dense").tooltip("Toggle object list")

    def _build_image_viewer(self) -> None:
        """Build the image container with the interactive image."""
        container = (
            ui.element("div")
            .classes("w-full h-full relative overflow-hidden")
            .style("display: flex; justify-content: center; align-items: center;")
        )

        with container:
            self.message_label = ui.label("Select an image from the tree").classes(
                "text-gray-400"
            )
            self.interactive_image = (
                ui.interactive_image("", cross=False)
                .classes("w-auto h-auto")
                .style(
                    "max-width: none; max-height: none; "
                    "transform-origin: center center;"
                )
            )
            self.interactive_image.visible = False

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
    # Image loading and overlay
    # ------------------------------------------------------------------

    def _load_image(self, image_path: Path) -> None:
        """Load an image and its labels, update the display."""
        logger.info(f"Loading image: {image_path}")
        self.message_label.visible = False
        self.interactive_image.visible = True

        # Open image
        self.image_object = Image.open(image_path)
        self.interactive_image.source = self.image_object

        # Resolve corresponding label file
        label_path = self.label_base_path / image_path.relative_to(
            self.image_base_path
        ).with_suffix(".txt")
        logger.info(f"Label file: {label_path}")

        # Parse labels
        self.current_labels = parse_yolo_labels(
            label_path, self.image_object.width, self.image_object.height
        )

        # Render overlay
        self._refresh_overlay()

        # Update object list in drawer
        self._update_object_list(label_path)

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

    def _update_object_list(self, label_path: Path) -> None:
        """Populate the right-drawer object list for the current image."""
        self.object_list_container.clear()

        if not self.current_labels:
            with self.object_list_container:
                msg = (
                    "No labels found"
                    if not label_path.exists()
                    else "No objects in label file"
                )
                ui.label(msg).classes("text-xs text-gray-400")
            return

        with self.object_list_container:
            for label_obj in self.current_labels:
                color_str = self.config.get_color_string(label_obj.class_id)
                class_name = self.class_map.get(
                    label_obj.class_id, f"class {label_obj.class_id}"
                )
                obj_type = "bbox" if label_obj.label_type == "bbox" else "poly"

                with ui.row().classes("items-center gap-2 w-full"):
                    # Color dot
                    ui.element("div").style(
                        f"width: 12px; height: 12px; border-radius: 50%; "
                        f"background: {color_str}; flex-shrink: 0;"
                    )
                    # Object info
                    ui.label(f"#{label_obj.index + 1} {class_name}").classes(
                        "text-white text-xs"
                    )
                    ui.label(f"[{obj_type}]").classes("text-gray-400 text-xs ml-auto")
