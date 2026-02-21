"""Shared test fixtures for YoDa tests."""

from __future__ import annotations

from pathlib import Path

import pytest
from PIL import Image


@pytest.fixture
def tmp_image_dir(tmp_path: Path) -> Path:
    """Create a temporary image directory with a small test image."""
    img_dir = tmp_path / "images" / "train"
    img_dir.mkdir(parents=True)
    img = Image.new("RGB", (640, 480), color=(128, 128, 128))
    img.save(img_dir / "test1.jpg")
    img.save(img_dir / "test2.png")
    return tmp_path / "images"


@pytest.fixture
def tmp_label_dir(tmp_path: Path) -> Path:
    """Create a temporary label directory with YOLO label files."""
    lbl_dir = tmp_path / "labels" / "train"
    lbl_dir.mkdir(parents=True)

    # Segmentation polygon (class 0, 5 points)
    (lbl_dir / "test1.txt").write_text(
        "0 0.1 0.2 0.3 0.2 0.3 0.8 0.1 0.8 0.05 0.5\n"
        "2 0.5 0.5 0.7 0.5 0.7 0.9 0.5 0.9\n"
    )

    # Bounding box (class 1, cx cy w h)
    (lbl_dir / "test2.txt").write_text("1 0.5 0.5 0.4 0.6\n")

    return tmp_path / "labels"


@pytest.fixture
def empty_label_file(tmp_path: Path) -> Path:
    """Create an empty label file."""
    f = tmp_path / "empty.txt"
    f.write_text("")
    return f


@pytest.fixture
def sample_color_map_yaml(tmp_path: Path) -> Path:
    """Create a sample color_map YAML file."""
    f = tmp_path / "color_map.yaml"
    f.write_text("0: [255, 0, 0]\n1: [0, 255, 0]\n2: [0, 0, 255]\n")
    return f


@pytest.fixture
def sample_class_info_yaml(tmp_path: Path) -> Path:
    """Create a sample class info YAML (Ultralytics format)."""
    f = tmp_path / "classes.yaml"
    f.write_text("names:\n  0: back_bumper\n  1: front_bumper\n  2: wheel\n")
    return f
