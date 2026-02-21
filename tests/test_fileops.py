"""Unit tests for yoda.fileops — file discovery and tree building."""

from __future__ import annotations

from pathlib import Path

from PIL import Image

from yoda.fileops import get_file_tree, get_files


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
    """Tests for get_file_tree()."""

    def test_tree_structure(self, tmp_image_dir: Path) -> None:
        """Tree should contain folders and image files."""
        tree = get_file_tree(tmp_image_dir)
        assert len(tree) == 1  # 'train' subfolder
        train_node = tree[0]
        assert train_node["label"] == "train"
        assert "children" in train_node
        children = train_node["children"]
        assert len(children) == 2  # test1.jpg, test2.png

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
