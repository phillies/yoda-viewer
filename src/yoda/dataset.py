"""Dataset utility functions for YoDa.

Provides helpers for loading class info from Ultralytics-format YAML files
and resolving dataset paths. The main UI class (YoDaBrowser) owns data loading
and display; this module contains reusable data-layer helpers.
"""

from pathlib import Path

import yaml
from loguru import logger


def load_class_map(class_info_yaml: Path | None) -> dict[int, str]:
    """Load a class name mapping from an Ultralytics-format YAML file.

    The YAML is expected to have a ``names`` key mapping integer class IDs
    to string class names, e.g.::

        names:
          0: back_bumper
          1: back_door
          ...

    Args:
        class_info_yaml: Path to the YAML file. If ``None`` or the file does
            not exist, an empty dict is returned.

    Returns:
        A dict mapping class ID (int) → class name (str).

    """
    if class_info_yaml is None or not class_info_yaml.exists():
        return {}

    try:
        with class_info_yaml.open("r", encoding="utf-8") as f:
            data = yaml.safe_load(f)
        class_map: dict[int, str] = data.get("names", {})
        logger.info(f"Loaded {len(class_map)} class names from {class_info_yaml}")
        return class_map
    except Exception as e:
        logger.error(f"Failed to load class info from {class_info_yaml}: {e}")
        return {}


def load_dataset(dataset_yaml: Path) -> None:
    """Load a full Ultralytics dataset YAML (not yet implemented).

    This will parse the dataset YAML to resolve train/val/test image and
    label paths. Reserved for a future version.
    """
    raise NotImplementedError(
        "load_dataset() is not yet implemented. "
        "Use YoDaConfig settings for image/label paths."
    )
