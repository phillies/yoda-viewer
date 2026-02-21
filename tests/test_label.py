"""Unit tests for yoda.label — parsing and SVG rendering."""

from __future__ import annotations

from pathlib import Path

import pytest

from yoda.label import LabelObject, parse_yolo_labels, render_labels_to_svg

# ---------------------------------------------------------------------------
# parse_yolo_labels
# ---------------------------------------------------------------------------


class TestParseYoloLabels:
    """Tests for the YOLO label parser."""

    def test_parse_segmentation_polygon(self, tmp_label_dir: Path) -> None:
        """Segmentation polygon lines produce polygon LabelObjects."""
        labels = parse_yolo_labels(tmp_label_dir / "train" / "test1.txt", 640, 480)
        assert len(labels) == 2

        # First object: class 0, polygon with 5 points
        obj = labels[0]
        assert obj.class_id == 0
        assert obj.label_type == "polygon"
        assert len(obj.pixel_points) == 5
        assert obj.index == 0
        # Pixel bbox should be computed from polygon extents
        bx, by, bw, bh = obj.pixel_bbox
        assert bw > 0
        assert bh > 0

    def test_parse_bounding_box(self, tmp_label_dir: Path) -> None:
        """A 4-value coordinate line produces a bbox LabelObject."""
        labels = parse_yolo_labels(tmp_label_dir / "train" / "test2.txt", 640, 480)
        assert len(labels) == 1
        obj = labels[0]
        assert obj.class_id == 1
        assert obj.label_type == "bbox"
        # cx=0.5, cy=0.5, w=0.4, h=0.6  →  pixel: x=192, y=96, w=256, h=288
        bx, by, bw, bh = obj.pixel_bbox
        assert bw == pytest.approx(0.4 * 640)
        assert bh == pytest.approx(0.6 * 480)

    def test_parse_empty_file(self, empty_label_file: Path) -> None:
        """An empty label file produces an empty list."""
        labels = parse_yolo_labels(empty_label_file, 640, 480)
        assert labels == []

    def test_parse_missing_file(self, tmp_path: Path) -> None:
        """A nonexistent file produces an empty list."""
        labels = parse_yolo_labels(tmp_path / "nonexistent.txt", 640, 480)
        assert labels == []

    def test_parse_multi_object_file(self, tmp_label_dir: Path) -> None:
        """A file with multiple lines produces multiple LabelObjects."""
        labels = parse_yolo_labels(tmp_label_dir / "train" / "test1.txt", 640, 480)
        assert len(labels) == 2
        assert labels[0].class_id == 0
        assert labels[1].class_id == 2

    def test_label_indices_are_sequential(self, tmp_label_dir: Path) -> None:
        """Object indices should match line positions."""
        labels = parse_yolo_labels(tmp_label_dir / "train" / "test1.txt", 640, 480)
        assert [l.index for l in labels] == [0, 1]

    def test_pixel_points_correctness(self, tmp_label_dir: Path) -> None:
        """Pixel points should be normalized coords * image dimensions."""
        labels = parse_yolo_labels(tmp_label_dir / "train" / "test1.txt", 640, 480)
        obj = labels[0]
        # First point: (0.1*640, 0.2*480) = (64, 96)
        assert obj.pixel_points[0] == pytest.approx((64.0, 96.0))

    def test_normalized_coords_preserved(self, tmp_label_dir: Path) -> None:
        """Raw normalized coordinates should be stored verbatim."""
        labels = parse_yolo_labels(tmp_label_dir / "train" / "test1.txt", 640, 480)
        obj = labels[0]
        assert obj.normalized_coords[:2] == pytest.approx([0.1, 0.2])


# ---------------------------------------------------------------------------
# render_labels_to_svg
# ---------------------------------------------------------------------------


class TestRenderLabelsToSvg:
    """Tests for the SVG renderer."""

    @pytest.fixture
    def sample_labels(self) -> list[LabelObject]:
        """Two sample labels for rendering tests."""
        return [
            LabelObject(
                index=0,
                class_id=0,
                label_type="polygon",
                normalized_coords=[0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
                pixel_points=[(64, 96), (192, 96), (192, 384)],
                pixel_bbox=(64, 96, 128, 288),
            ),
            LabelObject(
                index=1,
                class_id=1,
                label_type="bbox",
                normalized_coords=[0.5, 0.5, 0.4, 0.6],
                pixel_points=[(192, 96), (448, 384)],
                pixel_bbox=(192, 96, 256, 288),
            ),
        ]

    @pytest.fixture
    def color_map(self) -> dict[int, tuple[int, int, int]]:
        return {0: (255, 0, 0), 1: (0, 255, 0)}

    def test_segmask_only(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """With only show_segmask=True, only polygons are rendered."""
        svg = render_labels_to_svg(
            sample_labels, color_map, show_segmask=True, show_bbox=False
        )
        assert "<polygon" in svg
        # bbox object should NOT produce a <rect> since show_bbox=False
        assert "<rect" not in svg

    def test_bbox_only(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """With only show_bbox=True, bboxes produce rects."""
        svg = render_labels_to_svg(
            sample_labels, color_map, show_segmask=False, show_bbox=True
        )
        assert "<rect" in svg
        # Polygon's derived bbox uses dashed stroke
        assert "stroke-dasharray" in svg
        # No polygons
        assert "<polygon" not in svg

    def test_both_modes(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """Both modes on produces polygons and rects."""
        svg = render_labels_to_svg(
            sample_labels, color_map, show_segmask=True, show_bbox=True
        )
        assert "<polygon" in svg
        assert "<rect" in svg

    def test_nothing_shown(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """All toggles off produces empty SVG."""
        svg = render_labels_to_svg(
            sample_labels,
            color_map,
            show_segmask=False,
            show_bbox=False,
            show_class_id=False,
            show_class_name=False,
        )
        assert svg == ""

    def test_class_id_text(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """show_class_id adds text elements with the class ID."""
        svg = render_labels_to_svg(
            sample_labels,
            color_map,
            show_segmask=False,
            show_bbox=False,
            show_class_id=True,
        )
        assert "<text" in svg
        assert ">0</text>" in svg
        assert ">1</text>" in svg

    def test_class_name_text(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """show_class_name adds text elements with the class name."""
        class_map = {0: "bumper", 1: "wheel"}
        svg = render_labels_to_svg(
            sample_labels,
            color_map,
            show_segmask=False,
            show_bbox=False,
            show_class_name=True,
            class_map=class_map,
        )
        assert "bumper" in svg
        assert "wheel" in svg

    def test_uses_correct_colors(
        self,
        sample_labels: list[LabelObject],
    ) -> None:
        """Rendered SVG uses the provided color map."""
        custom_colors = {0: (10, 20, 30), 1: (40, 50, 60)}
        svg = render_labels_to_svg(
            sample_labels,
            custom_colors,
            show_segmask=True,
            show_bbox=True,
        )
        assert "rgb(10,20,30)" in svg
        assert "rgb(40,50,60)" in svg

    def test_invisible_labels_skipped(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """Labels with visible=False are not rendered."""
        sample_labels[0].visible = False
        svg = render_labels_to_svg(
            sample_labels, color_map, show_segmask=True, show_bbox=True
        )
        # Only the bbox label (index 1) should appear
        assert "rgb(255,0,0)" not in svg
        assert "rgb(0,255,0)" in svg

    def test_empty_labels(self) -> None:
        """Empty label list produces empty SVG."""
        svg = render_labels_to_svg([])
        assert svg == ""

    def test_class_id_and_name_combined(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """Both class ID and name shown together."""
        class_map = {0: "bumper", 1: "wheel"}
        svg = render_labels_to_svg(
            sample_labels,
            color_map,
            show_segmask=False,
            show_class_id=True,
            show_class_name=True,
            class_map=class_map,
        )
        assert "0 bumper" in svg
        assert "1 wheel" in svg
