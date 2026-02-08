from pathlib import Path

import yaml
from loguru import logger
from nicegui import events, ui
from nicegui.elements.interactive_image import InteractiveImage
from nicegui.elements.label import Label
from nicegui.elements.tree import Tree
from PIL import Image

import yoda.label
from yoda import fileops
from yoda.constants import COLOR_MAP_RGB_STRING


class YoDaBrowser:
    image_base_path: Path
    label_base_path: Path
    tree_data: list[dict[str, object]]
    tree: Tree
    interactive_image: InteractiveImage
    show_overlay: bool = True
    current_image_path: Path | None = None
    image_object: Image.Image | None = None
    color_info: str = ""
    segment_svg_content: str = ""
    class_map: dict[int, str] = {}
    label_legend: list[Label] = []

    def __init__(
        self,
        image_base_path: Path,
        label_base_path: Path,
        class_info_yaml: Path | None = None,
    ) -> None:
        self.image_base_path = image_base_path.resolve()
        self.label_base_path = label_base_path.resolve()
        if class_info_yaml and class_info_yaml.exists():
            with class_info_yaml.open("r", encoding="utf-8") as f:
                self.class_map = yaml.safe_load(f).get("names", {})
            logger.info(f"Loaded class info from {class_info_yaml}: {self.class_map}")
        self.tree_data = fileops.get_file_tree(self.image_base_path)

    def display_image_with_labels(self, image_file: Path) -> None:
        label_file = self.label_base_path / image_file.relative_to(
            self.image_base_path
        ).with_suffix(".txt")
        logger.info(f"Displaying image: {image_file}")
        logger.info(f"Corresponding label file: {label_file}")
        self.image_object = Image.open(image_file)
        self.segment_svg_content = yoda.label.parse_yolo(
            label_file, self.image_object.width, self.image_object.height
        )
        self.interactive_image.source = self.image_object
        self.interactive_image.content = self.segment_svg_content

    def render(self) -> None:
        """Renders the main UI layout."""
        # Use a splitter to divide tree and image view
        with ui.splitter(value=20).classes("w-full h-screen") as splitter:
            # Left Drawer: File Tree
            with splitter.before, ui.column().classes("w-full h-full p-2"):
                ui.label("Images").classes("text-lg font-bold")
                self.tree = ui.tree(self.tree_data, on_select=self.on_tree_select)
                self.tree.classes("w-full")

            # Right Content: Image Viewer and Controls
            with (
                splitter.after,
                ui.column()
                .classes("w-full h-full relative bg-gray-900")
                .style("overflow: hidden;"),
            ):
                # Toolbar / Header
                with ui.row().classes("w-full p-2 bg-gray-800 items-center gap-4 z-10"):
                    ui.label("Controls:").classes("text-white font-bold")
                    ui.switch(
                        "Show Overlay",
                        value=self.show_overlay,
                        on_change=self.toggle_overlay,
                    ).props("color=green")

                    ui.separator().props("vertical").classes("mx-2")
                    if len(self.class_map) == 0:
                        self.label_legend = [
                            ui.label("No classes loaded").classes(
                                "text-xs text-gray-400"
                            )
                        ]
                    else:
                        self.label_legend = [
                            ui.label(f"{name}")
                            .classes("text-xs")
                            .style(f"color: {COLOR_MAP_RGB_STRING[id]}")
                            for id, name in self.class_map.items()
                        ]

                # Image Container with absolute positioning for pan/zoom
                self.container = (
                    ui.element("div")
                    .classes("w-full h-full relative overflow-hidden")
                    .style(
                        "display: flex; justify-content: center; align-items: center;"
                    )
                )

                with self.container:
                    # Validation message or empty state
                    self.message_label = ui.label(
                        "Select an image from the tree"
                    ).classes("text-gray-400")

                    # The interactive image
                    self.interactive_image = (
                        ui.interactive_image(
                            "",
                            cross=False,  # Disable crosshair
                        )
                        .classes("w-auto h-auto")
                        .style(
                            "max-width: none; max-height: none; "
                            "transform-origin: center center;"
                        )
                    )
                    self.interactive_image.visible = False

    def on_tree_select(self, e: events.ValueChangeEventArguments) -> None:
        """Handle file selection from the tree."""
        if not e.value:
            return

        selected_path = Path(e.value)
        if selected_path.is_file():
            self.load_image(selected_path)

    def load_image(self, path: Path) -> None:
        """Loads the selected image and its overlay."""
        self.message_label.visible = False

        self.display_image_with_labels(path)
        self.interactive_image.visible = True

    def toggle_overlay(self, e: events.ValueChangeEventArguments) -> None:
        """Toggle the visibility of the label overlay."""
        self.show_overlay = e.value
        if self.image_object is None:
            return
        self.interactive_image.content = (
            self.segment_svg_content if self.show_overlay else ""
        )
