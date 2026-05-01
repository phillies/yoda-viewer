from pathlib import Path

_IMAGE_EXTENSIONS: frozenset[str] = frozenset(
    {".jpg", ".jpeg", ".png", ".bmp", ".webp"}
)

# Suffix appended to a directory node's ID to create a lazy-placeholder child.
# The placeholder is replaced with real children on first expansion.
LAZY_PLACEHOLDER_SUFFIX = "/__lazy__"


def get_files(base_dir: Path) -> list[Path]:
    """Recursively retrieves all files from the given base directory.

    Args:
        base_dir (Path): The directory to search for files.

    Returns:
        list[Path]: A list of file paths found in the directory and its subdirectories.

    """
    return [f for f in base_dir.rglob("*") if f.is_file()]


def get_dir_children(path: Path) -> list[dict[str, object]]:
    """Return immediate image-file and sub-directory children of *path*.

    Directories receive a single lazy-placeholder child so they appear
    expandable in the UI tree without scanning their contents up-front.
    The placeholder is identified by the :data:`LAZY_PLACEHOLDER_SUFFIX`
    suffix on its ``id`` field and is replaced with real children when the
    user expands the directory node.

    Args:
        path: The directory whose direct children to list.

    Returns:
        A list of tree nodes for direct children, directories before files,
        both groups sorted alphabetically.

    """
    nodes: list[dict[str, object]] = []

    if not path.exists() or not path.is_dir():
        return nodes

    items = sorted(path.iterdir(), key=lambda x: (not x.is_dir(), x.name.lower()))

    for item in items:
        if item.name.startswith("."):
            continue

        node: dict[str, object] = {
            "id": str(item.resolve()),
            "label": item.name,
        }

        if item.is_dir():
            # Placeholder child keeps the expand arrow visible; real children
            # are loaded on first expansion via get_dir_children().
            node["children"] = [
                {
                    "id": str(item.resolve()) + LAZY_PLACEHOLDER_SUFFIX,
                    "label": "...",
                }
            ]
            node["icon"] = "folder"
        elif item.suffix.lower() in _IMAGE_EXTENSIONS:
            node["icon"] = "image"
        else:
            continue

        nodes.append(node)

    return nodes


def get_file_tree(path: Path) -> list[dict[str, object]]:
    """Build the top-level tree for *path*.

    Only the direct children of *path* are scanned at startup; sub-directory
    contents are loaded lazily via :func:`get_dir_children` when the user
    expands a directory node in the UI.  This avoids scanning tens of
    thousands of files on startup when working with large datasets.

    Args:
        path: The root directory to start from.

    Returns:
        A list of nodes representing the top-level items in the directory.

    """
    return get_dir_children(path)
