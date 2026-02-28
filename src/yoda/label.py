from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path
from typing import Literal

from loguru import logger

from yoda.constants import COLOR_MAP


@dataclass
class LabelObject:
    """A single parsed YOLO label object (bounding box or polygon).

    Attributes:
        index: 0-based index of the object within its label file.
        class_id: YOLO class ID.
        label_type: Either "bbox" (4-value center format) or "polygon".
        normalized_coords: Raw normalized coordinates from the label file.
        pixel_points: For polygons — list of (x, y) pixel coordinate tuples.
        pixel_bbox: For all objects — axis-aligned bounding box as (x, y, w, h)
                    in pixel coordinates. For bbox type this is the direct conversion;
                    for polygons this is computed from min/max of points.

    """

    index: int
    class_id: int
    label_type: Literal["bbox", "polygon"]
    normalized_coords: list[float]
    pixel_points: list[tuple[float, float]] = field(default_factory=list)
    pixel_bbox: tuple[float, float, float, float] = (0.0, 0.0, 0.0, 0.0)
    # V2 extensibility: visibility toggle per object
    visible: bool = True


def parse_yolo_labels(
    file_path: Path,
    image_width: int,
    image_height: int,
) -> list[LabelObject]:
    """Parse a YOLO format label file into structured LabelObject instances.

    Args:
        file_path: Path to the YOLO .txt file.
        image_width: Width of the corresponding image in pixels.
        image_height: Height of the corresponding image in pixels.

    Returns:
        A list of LabelObject instances. Empty list if file doesn't exist or
        is empty.

    """
    if not file_path.exists():
        return []

    labels: list[LabelObject] = []

    try:
        with file_path.open("r") as f:
            for idx, line in enumerate(f):
                parts = line.strip().split()
                if not parts:
                    continue

                class_id = int(parts[0])
                coords = [float(x) for x in parts[1:]]

                if len(coords) == 4:
                    # Bounding Box: cx, cy, w, h (normalized)
                    cx, cy, w, h = coords
                    pixel_w = w * image_width
                    pixel_h = h * image_height
                    x = (cx * image_width) - (pixel_w / 2)
                    y = (cy * image_height) - (pixel_h / 2)

                    labels.append(
                        LabelObject(
                            index=idx,
                            class_id=class_id,
                            label_type="bbox",
                            normalized_coords=coords,
                            pixel_points=[(x, y), (x + pixel_w, y + pixel_h)],
                            pixel_bbox=(x, y, pixel_w, pixel_h),
                        )
                    )
                elif len(coords) >= 6:
                    # Segmentation polygon: x1 y1 x2 y2 ... xN yN (normalized)
                    points: list[tuple[float, float]] = []
                    for i in range(0, len(coords), 2):
                        if i + 1 < len(coords):
                            px = coords[i] * image_width
                            py = coords[i + 1] * image_height
                            points.append((px, py))

                    if len(points) >= 3:
                        xs = [p[0] for p in points]
                        ys = [p[1] for p in points]
                        min_x, max_x = min(xs), max(xs)
                        min_y, max_y = min(ys), max(ys)

                        labels.append(
                            LabelObject(
                                index=idx,
                                class_id=class_id,
                                label_type="polygon",
                                normalized_coords=coords,
                                pixel_points=points,
                                pixel_bbox=(
                                    min_x,
                                    min_y,
                                    max_x - min_x,
                                    max_y - min_y,
                                ),
                            )
                        )

    except Exception as e:
        logger.error(f"Error parsing YOLO file {file_path}: {e}")
        return []

    return labels


def _render_segmask(
    label: LabelObject, color_str: str, *, selected: bool = False
) -> str:
    """Render a segmentation mask polygon SVG element."""
    points_str = " ".join(f"{p[0]},{p[1]}" for p in label.pixel_points)
    if selected:
        # Marching ants selection with brighter fill
        return (
            f'<polygon points="{points_str}" '
            f'fill="{color_str}" stroke="white" '
            f'fill-opacity="0.5" stroke-width="3" '
            f'stroke-dasharray="8,3" '
            f'style="animation: marchingAnts 0.6s linear infinite;" />'
        )
    return (
        f'<polygon points="{points_str}" '
        f'fill="{color_str}" stroke="{color_str}" '
        f'fill-opacity="0.3" stroke-width="2" />'
    )


def _render_bbox(label: LabelObject, color_str: str) -> str:
    """Render a bounding box SVG element."""
    bx, by, bw, bh = label.pixel_bbox
    if label.label_type == "bbox":
        return (
            f'<rect x="{bx}" y="{by}" '
            f'width="{bw}" height="{bh}" '
            f'fill="{color_str}" stroke="{color_str}" '
            f'fill-opacity="0.2" stroke-width="2" />'
        )
    return (
        f'<rect x="{bx}" y="{by}" '
        f'width="{bw}" height="{bh}" '
        f'fill="none" stroke="{color_str}" '
        f'stroke-width="2" stroke-dasharray="6,3" />'
    )


def _render_text_label(
    label: LabelObject,
    color_str: str,  # noqa: ARG001
    *,
    show_class_id: bool,
    show_class_name: bool,
    class_map: dict[int, str],
) -> str:
    """Render class ID / class name SVG text elements."""
    tx = label.pixel_bbox[0]
    ty = label.pixel_bbox[1]

    text_parts: list[str] = []
    if show_class_id:
        text_parts.append(str(label.class_id))
    if show_class_name:
        name = class_map.get(label.class_id, f"cls_{label.class_id}")
        text_parts.append(name)
    text_content = " ".join(text_parts)

    text_w = max(len(text_content) * 6, 20) + 8
    text_h = 16
    pad_x = 4
    pad_y = 4
    bg = (
        f'<rect x="{tx}" y="{ty - text_h}" '
        f'width="{text_w}" height="{text_h}" '
        f'fill="rgba(0,0,0,0.7)" rx="3" />'
    )
    txt = (
        f'<text x="{tx + pad_x}" y="{ty - pad_y}" '
        f'fill="white" font-size="10" '
        f'font-family="-apple-system, BlinkMacSystemFont, sans-serif">'
        f"{text_content}</text>"
    )
    return bg + txt


def render_labels_to_svg(
    labels: list[LabelObject],
    color_map: dict[int, tuple[int, int, int]] | None = None,
    *,
    show_bbox: bool = False,
    show_segmask: bool = True,
    show_class_id: bool = False,
    show_class_name: bool = False,
    class_map: dict[int, str] | None = None,
    selected_index: int | None = None,
) -> str:
    """Render a list of LabelObject instances to an SVG string.

    Args:
        labels: Parsed label objects.
        color_map: Mapping of class_id -> (r, g, b) tuples. Falls back to
                   the default COLOR_MAP from constants.
        show_bbox: Whether to render bounding boxes.
        show_segmask: Whether to render segmentation mask polygons.
        show_class_id: Whether to render class ID text labels.
        show_class_name: Whether to render class name text labels.
        class_map: Mapping of class_id -> class name string.
        selected_index: Index of the currently-selected label (highlighted).

    Returns:
        An SVG string containing all rendered elements.

    """
    if color_map is None:
        color_map = COLOR_MAP
    if class_map is None:
        class_map = {}

    svg_elements: list[str] = []

    for label in labels:
        if not label.visible:
            continue

        color = color_map.get(label.class_id % len(color_map), (255, 255, 255))
        color_str = f"rgb({color[0]},{color[1]},{color[2]})"
        is_selected = label.index == selected_index

        if show_segmask and label.label_type == "polygon":
            svg_elements.append(_render_segmask(label, color_str, selected=is_selected))

        if show_bbox:
            svg_elements.append(_render_bbox(label, color_str))

        if show_class_id or show_class_name:
            svg_elements.append(
                _render_text_label(
                    label,
                    color_str,
                    show_class_id=show_class_id,
                    show_class_name=show_class_name,
                    class_map=class_map,
                )
            )

    return "".join(svg_elements)


def write_yolo_labels(
    file_path: Path,
    labels: list[LabelObject],
) -> None:
    """Write label objects back to a YOLO-format text file.

    Each label is written as one line: ``<class_id> <normalized_coords...>``.
    The original normalised coordinates stored in
    :pyattr:`LabelObject.normalized_coords` are used so that no precision
    is lost through pixel → normalised round-trips.

    Args:
        file_path: Destination ``.txt`` file path.
        labels: The label objects to serialise.

    """
    file_path.parent.mkdir(parents=True, exist_ok=True)
    with file_path.open("w", encoding="utf-8") as fh:
        for label in labels:
            coords_str = " ".join(f"{c:.6f}" for c in label.normalized_coords)
            fh.write(f"{label.class_id} {coords_str}\n")
    logger.info(f"Saved {len(labels)} labels to {file_path}")


def create_label_from_pixels(
    pixel_points: list[tuple[float, float]],
    image_width: int,
    image_height: int,
    class_id: int,
    index: int,
) -> LabelObject:
    """Create a new polygon LabelObject from pixel coordinates.

    Converts pixel coordinates to normalised YOLO format and computes the
    bounding box. Requires at least 3 points for a valid polygon.

    Args:
        pixel_points: List of (x, y) pixel coordinate tuples forming
            the polygon.
        image_width: Width of the image in pixels.
        image_height: Height of the image in pixels.
        class_id: YOLO class ID for the new object.
        index: 0-based index for the new object.

    Returns:
        A new LabelObject with ``label_type="polygon"``.

    Raises:
        ValueError: If fewer than 3 points are provided or image
            dimensions are non-positive.

    """
    if len(pixel_points) < 3:
        raise ValueError(
            f"At least 3 points required for a polygon, got {len(pixel_points)}"
        )
    if image_width <= 0 or image_height <= 0:
        raise ValueError(
            f"Image dimensions must be positive, got {image_width}x{image_height}"
        )

    # Build normalised coordinates
    normalized_coords: list[float] = []
    for px, py in pixel_points:
        normalized_coords.append(px / image_width)
        normalized_coords.append(py / image_height)

    # Compute bounding box from pixel points
    xs = [p[0] for p in pixel_points]
    ys = [p[1] for p in pixel_points]
    min_x, max_x = min(xs), max(xs)
    min_y, max_y = min(ys), max(ys)

    return LabelObject(
        index=index,
        class_id=class_id,
        label_type="polygon",
        normalized_coords=normalized_coords,
        pixel_points=list(pixel_points),
        pixel_bbox=(min_x, min_y, max_x - min_x, max_y - min_y),
    )


def delete_label(
    labels: list[LabelObject],
    label_index: int,
) -> list[LabelObject]:
    """Remove a label by its index and re-number remaining labels.

    Args:
        labels: The current list of label objects.
        label_index: The ``index`` attribute of the label to remove.

    Returns:
        A new list with the label removed and indices renumbered
        sequentially starting from 0.

    """
    new_labels = [lbl for lbl in labels if lbl.index != label_index]
    for i, lbl in enumerate(new_labels):
        lbl.index = i
    return new_labels


# Backward-compatible wrapper (deprecated)
def parse_yolo(file_path: Path, image_width: int, image_height: int) -> str:
    """Legacy function: parse and render in one step.

    Deprecated — use parse_yolo_labels() + render_labels_to_svg() instead.
    """
    labels = parse_yolo_labels(file_path, image_width, image_height)
    return render_labels_to_svg(labels, show_segmask=True)
