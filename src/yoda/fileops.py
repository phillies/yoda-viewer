from pathlib import Path


def get_files(base_dir: Path) -> list[Path]:
    """Recursively retrieves all files from the given base directory.

    Args:
        base_dir (Path): The directory to search for files.

    Returns:
        list[Path]: A list of file paths found in the directory and its subdirectories.

    """
    return [f for f in base_dir.rglob("*") if f.is_file()]


def get_file_tree(path: Path) -> list[dict[str, object]]:
    """Recursively builds a dictionary structure for the NiceGUI tree component.

    Args:
        path (Path): The root directory to start from.

    Returns:
        list[dict[str, object]]: A list of nodes representing the directory structure.

    """
    tree: list[dict[str, object]] = []

    if not path.exists() or not path.is_dir():
        return tree

    # Sort items: directories first, then files
    items = sorted(path.iterdir(), key=lambda x: (not x.is_dir(), x.name.lower()))

    for item in items:
        # Skip hidden files
        if item.name.startswith("."):
            continue

        node = {
            "id": str(item.resolve()),
            "label": item.name,
        }

        if item.is_dir():
            children = get_file_tree(item)
            # Only add directories if they contain files or other directories
            # But the user might want empty folders too. Let's keep them for now.
            node["children"] = children
            node["icon"] = "folder"
        # Check for image extensions
        elif item.suffix.lower() in [".jpg", ".jpeg", ".png", ".bmp", ".webp"]:
            node["icon"] = "image"
        else:
            continue  # Skip non-image files in the tree view for now?
            # Requirement said "folder tree of the images".
            # Assuming we only want to show images and folders.

        tree.append(node)

    return tree
