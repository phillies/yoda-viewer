"""Unit tests for yoda.fileops — file discovery and tree building."""

from __future__ import annotations

from pathlib import Path

import pytest
from PIL import Image

from yoda.fileops import (
    LAZY_PLACEHOLDER_SUFFIX,
    get_dir_children,
    get_file_tree,
    get_files,
)


class TestGetFiles:
    """Tests for get_files()."""

    def test_finds_all_files(self, tmp_image_dir: Path) -> None:
        """Should find all files recursively."""
        files = get_files(tmp_image_dir)
        assert len(files) == 2
        names = {f.name for f in files}
        assert "test1.jpg" in names
        assert "test2.png" in names

    def test_empty_dir(self, tmp_path: Path) -> None:
        """Empty directory returns empty list."""
        empty = tmp_path / "empty"
        empty.mkdir()
        assert get_files(empty) == []

    def test_nested_structure(self, tmp_path: Path) -> None:
        """Files in nested subdirectories are found."""
        (tmp_path / "a" / "b" / "c").mkdir(parents=True)
        (tmp_path / "a" / "b" / "c" / "deep.jpg").write_bytes(b"x")
        (tmp_path / "a" / "top.txt").write_bytes(b"y")
        files = get_files(tmp_path / "a")
        assert len(files) == 2


class TestGetFileTree:
    """Tests for get_file_tree() — top-level lazy-load aware tree."""

    def test_tree_structure(self, tmp_image_dir: Path) -> None:
        """Top-level tree should contain the 'train' folder with a lazy placeholder."""
        tree = get_file_tree(tmp_image_dir)
        assert len(tree) == 1  # 'train' subfolder only
        train_node = tree[0]
        assert train_node["label"] == "train"
        assert "children" in train_node
        children = train_node["children"]
        # Lazy placeholder — real children are loaded on expansion
        assert len(children) == 1
        assert str(children[0]["id"]).endswith(LAZY_PLACEHOLDER_SUFFIX)

    def test_skips_non_image_files(self, tmp_path: Path) -> None:
        """Non-image files are excluded from the tree."""
        d = tmp_path / "dir"
        d.mkdir()
        (d / "data.txt").write_text("hello")
        (d / "photo.jpg").write_bytes(b"\xff\xd8\xff\xe0")
        tree = get_file_tree(d)
        labels = [n["label"] for n in tree]
        assert "photo.jpg" in labels
        assert "data.txt" not in labels

    def test_skips_hidden_files(self, tmp_path: Path) -> None:
        """Hidden files (starting with .) are skipped."""
        d = tmp_path / "dir"
        d.mkdir()
        (d / ".hidden.jpg").write_bytes(b"\xff")
        (d / "visible.png").write_bytes(b"\xff")
        tree = get_file_tree(d)
        labels = [n["label"] for n in tree]
        assert ".hidden.jpg" not in labels
        assert "visible.png" in labels

    def test_nonexistent_dir(self, tmp_path: Path) -> None:
        """Nonexistent directory returns empty tree."""
        tree = get_file_tree(tmp_path / "nope")
        assert tree == []

    def test_directories_first(self, tmp_path: Path) -> None:
        """Directories should appear before files in the tree."""
        d = tmp_path / "root"
        d.mkdir()
        (d / "z_image.jpg").write_bytes(b"\xff")
        (d / "a_folder").mkdir()
        img = Image.new("RGB", (10, 10))
        img.save(d / "a_folder" / "inner.png")
        tree = get_file_tree(d)
        assert tree[0]["label"] == "a_folder"
        assert tree[1]["label"] == "z_image.jpg"


class TestGetDirChildren:
    """Tests for get_dir_children() — single-level lazy-loading helper."""

    def test_lists_images_in_dir(self, tmp_image_dir: Path) -> None:
        """Direct children of a directory are returned (no recursion)."""
        train_dir = tmp_image_dir / "train"
        children = get_dir_children(train_dir)
        assert len(children) == 2
        labels = {c["label"] for c in children}
        assert "test1.jpg" in labels
        assert "test2.png" in labels

    def test_dir_gets_lazy_placeholder(self, tmp_path: Path) -> None:
        """Sub-directory children should have one lazy-placeholder child."""
        parent = tmp_path / "parent"
        child_dir = parent / "subdir"
        child_dir.mkdir(parents=True)
        # File validity is irrelevant here — only the extension matters.
        (child_dir / "img.jpg").write_bytes(b"\xff")

        nodes = get_dir_children(parent)
        assert len(nodes) == 1
        node = nodes[0]
        assert node["label"] == "subdir"
        sub_children = node["children"]  # type: ignore[index]
        assert len(sub_children) == 1
        assert str(sub_children[0]["id"]).endswith(LAZY_PLACEHOLDER_SUFFIX)

    def test_nonexistent_dir_returns_empty(self, tmp_path: Path) -> None:
        """Non-existent path returns an empty list."""
        assert get_dir_children(tmp_path / "missing") == []

    def test_skips_hidden_and_non_image(self, tmp_path: Path) -> None:
        """Hidden files and non-image files are excluded."""
        d = tmp_path / "d"
        d.mkdir()
        (d / ".hidden.jpg").write_bytes(b"x")
        (d / "notes.txt").write_text("hi")
        (d / "photo.png").write_bytes(b"x")
        nodes = get_dir_children(d)
        labels = [n["label"] for n in nodes]
        assert labels == ["photo.png"]

    def test_directories_before_files(self, tmp_path: Path) -> None:
        """Directories are sorted before image files."""
        d = tmp_path / "d"
        d.mkdir()
        (d / "z.jpg").write_bytes(b"x")
        (d / "aaa").mkdir()
        nodes = get_dir_children(d)
        assert nodes[0]["label"] == "aaa"
        assert nodes[1]["label"] == "z.jpg"

    @pytest.mark.parametrize("ext", [".jpg", ".jpeg", ".png", ".bmp", ".webp"])
    def test_all_image_extensions_included(self, tmp_path: Path, ext: str) -> None:
        """All supported image extensions are recognised."""
        d = tmp_path / "d"
        d.mkdir()
        (d / f"img{ext}").write_bytes(b"x")
        nodes = get_dir_children(d)
        assert len(nodes) == 1
        assert nodes[0]["label"] == f"img{ext}"
