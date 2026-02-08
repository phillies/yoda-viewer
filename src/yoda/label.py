from pathlib import Path

from loguru import logger

from yoda.constants import COLOR_MAP


def parse_yolo(file_path: Path, image_width: int, image_height: int) -> str:
    """Parses a YOLO format text file and converts it to an SVG polygon string.

    Args:
        file_path (Path): Path to the YOLO .txt file.
        image_width (int): Width of the corresponding image.
        image_height (int): Height of the corresponding image.

    Returns:
        str: An SVG string containing polygons for the segmentation masks.
             Returns an empty string if the file doesn't exist or is empty.

    """
    if not file_path.exists():
        return ""

    svg_elements = []

    # YOLO colors for different classes (just a simple palette for now)
    colors = COLOR_MAP

    try:
        with file_path.open("r") as f:
            for line in f:
                parts = line.strip().split()
                if not parts:
                    continue

                class_id = int(parts[0])
                coords = [float(x) for x in parts[1:]]

                # Pair up x, y coordinates
                points = []
                for i in range(0, len(coords), 2):
                    if i + 1 < len(coords):
                        x = coords[i] * image_width
                        y = coords[i + 1] * image_height
                        points.append(f"{x},{y}")

                if len(coords) == 4:
                    # Bounding Box: cx, cy, w, h (normalized)
                    cx, cy, w, h = coords
                    # Convert to top-left x, y and pixel dimensions
                    pixel_w = w * image_width
                    pixel_h = h * image_height
                    x = (cx * image_width) - (pixel_w / 2)
                    y = (cy * image_height) - (pixel_h / 2)

                    color = colors[class_id % len(colors)]
                    color_string = f"rgb({color[0]},{color[1]},{color[2]})"
                    svg_elements.append(
                        f'<rect x="{x}" y="{y}" width="{pixel_w}" height="{pixel_h}" '
                        f'fill="{color_string}" stroke="{color_string}" '
                        'fill-opacity="0.2" stroke-width="2" />'
                    )
                elif len(points) >= 3:  # Polygon needs at least 3 points
                    color = colors[class_id % len(colors)]
                    color_string = f"rgb({color[0]},{color[1]},{color[2]})"
                    points_str = " ".join(points)
                    # Create an SVG polygon
                    svg_elements.append(
                        f'<polygon points="{points_str}" fill="{color_string}" '
                        f'stroke="{color_string}" '
                        f'fill-opacity="0.4" stroke-width="2" />'
                    )

    except Exception as e:
        logger.error(f"Error parsing YOLO file {file_path}: {e}")
        return ""

    return "".join(svg_elements)
