"""Unit tests for yoda.label — parsing and SVG rendering."""

from __future__ import annotations

from pathlib import Path

import pytest

from yoda.label import (
    LabelObject,
    create_label_from_pixels,
    delete_label,
    parse_yolo_labels,
    render_labels_to_svg,
    write_yolo_labels,
)

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

    def test_selected_index_highlights_polygon(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """Selected polygon uses distinct SVG attributes (white stroke, dashed)."""
        svg = render_labels_to_svg(
            sample_labels,
            color_map,
            show_segmask=True,
            selected_index=0,
        )
        assert 'stroke="white"' in svg
        assert "stroke-dasharray" in svg

    def test_selected_index_none_no_highlight(
        self,
        sample_labels: list[LabelObject],
        color_map: dict[int, tuple[int, int, int]],
    ) -> None:
        """No selected index means no white dash stroke."""
        svg = render_labels_to_svg(
            sample_labels,
            color_map,
            show_segmask=True,
            selected_index=None,
        )
        assert 'stroke="white"' not in svg
        assert "stroke-dasharray" not in svg


# ---------------------------------------------------------------------------
# write_yolo_labels
# ---------------------------------------------------------------------------


class TestWriteYoloLabels:
    """Tests for the YOLO label writer."""

    def test_roundtrip_polygon(self, tmp_path: Path) -> None:
        """Writing then reading a polygon label preserves data."""
        labels = [
            LabelObject(
                index=0,
                class_id=2,
                label_type="polygon",
                normalized_coords=[0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
                pixel_points=[(64, 96), (192, 96), (192, 384)],
                pixel_bbox=(64, 96, 128, 288),
            ),
        ]
        out = tmp_path / "out.txt"
        write_yolo_labels(out, labels)

        reparsed = parse_yolo_labels(out, 640, 480)
        assert len(reparsed) == 1
        assert reparsed[0].class_id == 2
        assert reparsed[0].label_type == "polygon"
        assert reparsed[0].normalized_coords == pytest.approx(
            [0.1, 0.2, 0.3, 0.2, 0.3, 0.8], abs=1e-5
        )

    def test_roundtrip_bbox(self, tmp_path: Path) -> None:
        """Writing then reading a bbox label preserves data."""
        labels = [
            LabelObject(
                index=0,
                class_id=1,
                label_type="bbox",
                normalized_coords=[0.5, 0.5, 0.4, 0.6],
                pixel_points=[(192, 96), (448, 384)],
                pixel_bbox=(192, 96, 256, 288),
            ),
        ]
        out = tmp_path / "out.txt"
        write_yolo_labels(out, labels)

        reparsed = parse_yolo_labels(out, 640, 480)
        assert len(reparsed) == 1
        assert reparsed[0].class_id == 1
        assert reparsed[0].label_type == "bbox"

    def test_class_change_roundtrip(self, tmp_path: Path) -> None:
        """Changing class_id and writing preserves the new class."""
        labels = [
            LabelObject(
                index=0,
                class_id=0,
                label_type="polygon",
                normalized_coords=[0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
                pixel_points=[(64, 96), (192, 96), (192, 384)],
                pixel_bbox=(64, 96, 128, 288),
            ),
        ]
        # Simulate class change (V2 feature)
        labels[0].class_id = 5
        out = tmp_path / "out.txt"
        write_yolo_labels(out, labels)

        reparsed = parse_yolo_labels(out, 640, 480)
        assert reparsed[0].class_id == 5

    def test_write_creates_parent_dirs(self, tmp_path: Path) -> None:
        """Write creates parent directories if they don't exist."""
        out = tmp_path / "sub" / "dir" / "out.txt"
        write_yolo_labels(out, [])
        assert out.exists()
        assert out.read_text() == ""

    def test_write_multiple_labels(self, tmp_path: Path) -> None:
        """Multiple labels are written as separate lines."""
        labels = [
            LabelObject(
                index=0,
                class_id=0,
                label_type="polygon",
                normalized_coords=[0.1, 0.2, 0.3, 0.4, 0.5, 0.6],
                pixel_points=[(64, 96), (192, 192), (320, 288)],
                pixel_bbox=(64, 96, 256, 192),
            ),
            LabelObject(
                index=1,
                class_id=3,
                label_type="bbox",
                normalized_coords=[0.5, 0.5, 0.4, 0.6],
                pixel_points=[(192, 96), (448, 384)],
                pixel_bbox=(192, 96, 256, 288),
            ),
        ]
        out = tmp_path / "out.txt"
        write_yolo_labels(out, labels)

        lines = out.read_text().strip().splitlines()
        assert len(lines) == 2
        assert lines[0].startswith("0 ")
        assert lines[1].startswith("3 ")


# ---------------------------------------------------------------------------
# create_label_from_pixels (V3)
# ---------------------------------------------------------------------------


class TestCreateLabelFromPixels:
    """Tests for creating a LabelObject from pixel coordinates."""

    def test_basic_triangle(self) -> None:
        """A triangle of pixel coords produces a valid polygon label."""
        points = [(100.0, 100.0), (200.0, 100.0), (150.0, 200.0)]
        label = create_label_from_pixels(points, 640, 480, class_id=2, index=0)

        assert label.label_type == "polygon"
        assert label.class_id == 2
        assert label.index == 0
        assert len(label.pixel_points) == 3
        assert label.pixel_points == points

    def test_normalized_coords_correct(self) -> None:
        """Normalised coordinates are pixel coords / image dimensions."""
        points = [(64.0, 96.0), (192.0, 96.0), (192.0, 384.0)]
        label = create_label_from_pixels(points, 640, 480, class_id=0, index=0)

        assert label.normalized_coords == pytest.approx([0.1, 0.2, 0.3, 0.2, 0.3, 0.8])

    def test_bounding_box_computed(self) -> None:
        """Pixel bounding box should be the min/max extent of points."""
        points = [(50.0, 100.0), (200.0, 50.0), (150.0, 300.0)]
        label = create_label_from_pixels(points, 640, 480, class_id=0, index=0)

        bx, by, bw, bh = label.pixel_bbox
        assert bx == pytest.approx(50.0)
        assert by == pytest.approx(50.0)
        assert bw == pytest.approx(150.0)
        assert bh == pytest.approx(250.0)

    def test_too_few_points_raises(self) -> None:
        """Fewer than 3 points should raise ValueError."""
        with pytest.raises(ValueError, match="At least 3 points"):
            create_label_from_pixels(
                [(10.0, 10.0), (20.0, 20.0)], 640, 480, class_id=0, index=0
            )

    def test_zero_dimensions_raises(self) -> None:
        """Zero or negative image dimensions should raise ValueError."""
        points = [(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)]
        with pytest.raises(ValueError, match="positive"):
            create_label_from_pixels(points, 0, 480, class_id=0, index=0)
        with pytest.raises(ValueError, match="positive"):
            create_label_from_pixels(points, 640, -1, class_id=0, index=0)

    def test_roundtrip_through_write_and_read(self, tmp_path: Path) -> None:
        """A label created from pixels can be written and re-read."""
        points = [(64.0, 96.0), (192.0, 96.0), (192.0, 384.0)]
        label = create_label_from_pixels(points, 640, 480, class_id=3, index=0)

        out = tmp_path / "new.txt"
        write_yolo_labels(out, [label])

        reparsed = parse_yolo_labels(out, 640, 480)
        assert len(reparsed) == 1
        assert reparsed[0].class_id == 3
        assert reparsed[0].label_type == "polygon"
        assert len(reparsed[0].pixel_points) == 3
        # Check pixel points match within tolerance
        for orig, parsed in zip(points, reparsed[0].pixel_points):
            assert orig[0] == pytest.approx(parsed[0], abs=0.1)
            assert orig[1] == pytest.approx(parsed[1], abs=0.1)

    def test_visible_defaults_to_true(self) -> None:
        """New labels should be visible by default."""
        points = [(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)]
        label = create_label_from_pixels(points, 640, 480, class_id=0, index=0)
        assert label.visible is True


# ---------------------------------------------------------------------------
# delete_label (V3)
# ---------------------------------------------------------------------------


class TestDeleteLabel:
    """Tests for deleting labels and re-indexing."""

    def _make_labels(self, n: int) -> list[LabelObject]:
        """Create *n* dummy polygon labels."""
        return [
            LabelObject(
                index=i,
                class_id=i % 3,
                label_type="polygon",
                normalized_coords=[0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
                pixel_points=[(64, 96), (192, 96), (192, 384)],
                pixel_bbox=(64, 96, 128, 288),
            )
            for i in range(n)
        ]

    def test_delete_middle(self) -> None:
        """Deleting the middle label removes it and renumbers."""
        labels = self._make_labels(3)
        result = delete_label(labels, 1)
        assert len(result) == 2
        assert [l.index for l in result] == [0, 1]
        # Original class_ids: 0, 1, 2 → after removing index 1 → 0, 2
        assert result[0].class_id == 0
        assert result[1].class_id == 2

    def test_delete_first(self) -> None:
        """Deleting the first label re-indexes from 0."""
        labels = self._make_labels(3)
        result = delete_label(labels, 0)
        assert len(result) == 2
        assert [l.index for l in result] == [0, 1]

    def test_delete_last(self) -> None:
        """Deleting the last label works correctly."""
        labels = self._make_labels(3)
        result = delete_label(labels, 2)
        assert len(result) == 2
        assert [l.index for l in result] == [0, 1]

    def test_delete_nonexistent_index(self) -> None:
        """Deleting an index that doesn't exist returns all labels renumbered."""
        labels = self._make_labels(3)
        result = delete_label(labels, 99)
        assert len(result) == 3
        assert [l.index for l in result] == [0, 1, 2]

    def test_delete_from_single(self) -> None:
        """Deleting the only label produces an empty list."""
        labels = self._make_labels(1)
        result = delete_label(labels, 0)
        assert result == []

    def test_delete_from_empty(self) -> None:
        """Deleting from an empty list returns an empty list."""
        result = delete_label([], 0)
        assert result == []

    def test_delete_roundtrip(self, tmp_path: Path) -> None:
        """Delete + write + re-read preserves remaining labels."""
        labels = self._make_labels(3)
        result = delete_label(labels, 1)

        out = tmp_path / "deleted.txt"
        write_yolo_labels(out, result)

        reparsed = parse_yolo_labels(out, 640, 480)
        assert len(reparsed) == 2
        assert reparsed[0].class_id == 0
        assert reparsed[1].class_id == 2
