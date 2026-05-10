use std::fs;
use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PixelBBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PixelBBox {
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LabelType {
    Bbox,
    Polygon,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelObject {
    pub index: usize,
    pub class_id: u32,
    pub label_type: LabelType,
    pub normalized_coords: Vec<f32>,
    pub pixel_points: Vec<Point>,
    pub pixel_bbox: PixelBBox,
    pub visible: bool,
}

#[derive(Debug, Error)]
pub enum LabelError {
    #[error("line {line}: missing class id")]
    MissingClassId { line: usize },
    #[error("line {line}: invalid class id {value:?}")]
    InvalidClassId { line: usize, value: String },
    #[error("line {line}: invalid coordinate count {count}")]
    InvalidCoordinateCount { line: usize, count: usize },
    #[error("line {line}: invalid coordinate {value:?}")]
    InvalidCoordinate { line: usize, value: String },
    #[error("line {line}: polygon needs at least 3 points")]
    TooFewPolygonPoints { line: usize },
    #[error("image dimensions must be positive, got {width}x{height}")]
    InvalidImageDimensions { width: u32, height: u32 },
    #[error("at least 3 points required for a polygon, got {count}")]
    TooFewCreatePoints { count: usize },
    #[error(transparent)]
    Io(#[from] io::Error),
}

pub fn parse_yolo_labels(
    file_path: impl AsRef<Path>,
    image_width: u32,
    image_height: u32,
) -> Vec<LabelObject> {
    let file_path = file_path.as_ref();
    if !file_path.exists() {
        return Vec::new();
    }

    let Ok(contents) = fs::read_to_string(file_path) else {
        return Vec::new();
    };

    parse_yolo_label_text(&contents, image_width, image_height).unwrap_or_default()
}

fn parse_yolo_label_text(
    contents: &str,
    image_width: u32,
    image_height: u32,
) -> Result<Vec<LabelObject>, LabelError> {
    validate_dimensions(image_width, image_height)?;

    let mut labels = Vec::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line_number = line_index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let class_token = parts
            .next()
            .ok_or(LabelError::MissingClassId { line: line_number })?;
        let class_id = class_token.parse::<u32>().map_err(|_| LabelError::InvalidClassId {
            line: line_number,
            value: class_token.to_string(),
        })?;

        let mut coords = Vec::new();
        for part in parts {
            let coord = part.parse::<f32>().map_err(|_| LabelError::InvalidCoordinate {
                line: line_number,
                value: part.to_string(),
            })?;
            coords.push(coord);
        }

        match coords.len() {
            4 => labels.push(parse_bbox(line_index, class_id, coords, image_width, image_height)),
            count if count >= 6 && count % 2 == 0 => labels.push(parse_polygon(
                line_index,
                class_id,
                coords,
                image_width,
                image_height,
                line_number,
            )?),
            count => {
                return Err(LabelError::InvalidCoordinateCount {
                    line: line_number,
                    count,
                });
            }
        }
    }

    Ok(labels)
}

fn parse_bbox(
    index: usize,
    class_id: u32,
    coords: Vec<f32>,
    image_width: u32,
    image_height: u32,
) -> LabelObject {
    let cx = coords[0];
    let cy = coords[1];
    let width = coords[2] * image_width as f32;
    let height = coords[3] * image_height as f32;
    let x = (cx * image_width as f32) - (width / 2.0);
    let y = (cy * image_height as f32) - (height / 2.0);

    LabelObject {
        index,
        class_id,
        label_type: LabelType::Bbox,
        normalized_coords: coords,
        pixel_points: vec![Point::new(x, y), Point::new(x + width, y + height)],
        pixel_bbox: PixelBBox::new(x, y, width, height),
        visible: true,
    }
}

fn parse_polygon(
    index: usize,
    class_id: u32,
    coords: Vec<f32>,
    image_width: u32,
    image_height: u32,
    line_number: usize,
) -> Result<LabelObject, LabelError> {
    let points = coords
        .chunks_exact(2)
        .map(|chunk| Point::new(chunk[0] * image_width as f32, chunk[1] * image_height as f32))
        .collect::<Vec<_>>();

    if points.len() < 3 {
        return Err(LabelError::TooFewPolygonPoints { line: line_number });
    }

    let pixel_bbox = bounding_box_for_points(&points);
    Ok(LabelObject {
        index,
        class_id,
        label_type: LabelType::Polygon,
        normalized_coords: coords,
        pixel_points: points,
        pixel_bbox,
        visible: true,
    })
}

pub fn write_yolo_labels(
    file_path: impl AsRef<Path>,
    labels: &[LabelObject],
) -> Result<(), LabelError> {
    let file_path = file_path.as_ref();
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(file_path, serialize_yolo_labels(labels))?;
    Ok(())
}

fn serialize_yolo_labels(labels: &[LabelObject]) -> String {
    labels
        .iter()
        .map(|label| {
            let coords = label
                .normalized_coords
                .iter()
                .map(|coord| format!("{coord:.6}"))
                .collect::<Vec<_>>()
                .join(" ");
            format!("{} {coords}", label.class_id)
        })
        .collect::<Vec<_>>()
        .join("\n")
        + if labels.is_empty() { "" } else { "\n" }
}

pub fn create_label_from_pixels(
    pixel_points: &[(f32, f32)],
    image_width: u32,
    image_height: u32,
    class_id: u32,
    index: usize,
) -> Result<LabelObject, LabelError> {
    validate_dimensions(image_width, image_height)?;
    if pixel_points.len() < 3 {
        return Err(LabelError::TooFewCreatePoints {
            count: pixel_points.len(),
        });
    }

    let points = pixel_points
        .iter()
        .map(|(x, y)| Point::new(*x, *y))
        .collect::<Vec<_>>();
    let normalized_coords = points
        .iter()
        .flat_map(|point| {
            [
                point.x / image_width as f32,
                point.y / image_height as f32,
            ]
        })
        .collect::<Vec<_>>();

    Ok(LabelObject {
        index,
        class_id,
        label_type: LabelType::Polygon,
        normalized_coords,
        pixel_points: points.clone(),
        pixel_bbox: bounding_box_for_points(&points),
        visible: true,
    })
}

pub fn delete_label(labels: &[LabelObject], label_index: usize) -> Vec<LabelObject> {
    labels
        .iter()
        .filter(|label| label.index != label_index)
        .cloned()
        .enumerate()
        .map(|(new_index, mut label)| {
            label.index = new_index;
            label
        })
        .collect()
}

fn bounding_box_for_points(points: &[Point]) -> PixelBBox {
    let min_x = points.iter().map(|point| point.x).fold(f32::INFINITY, f32::min);
    let max_x = points
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|point| point.y).fold(f32::INFINITY, f32::min);
    let max_y = points
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);

    PixelBBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn validate_dimensions(image_width: u32, image_height: u32) -> Result<(), LabelError> {
    if image_width == 0 || image_height == 0 {
        return Err(LabelError::InvalidImageDimensions {
            width: image_width,
            height: image_height,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        create_label_from_pixels, delete_label, parse_yolo_label_text, parse_yolo_labels,
        write_yolo_labels, LabelObject, LabelType, PixelBBox, Point,
    };

    fn approx_eq(left: f32, right: f32) {
        assert!((left - right).abs() < 1e-4, "left={left}, right={right}");
    }

    fn sample_polygon() -> LabelObject {
        LabelObject {
            index: 0,
            class_id: 2,
            label_type: LabelType::Polygon,
            normalized_coords: vec![0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
            pixel_points: vec![
                Point::new(64.0, 96.0),
                Point::new(192.0, 96.0),
                Point::new(192.0, 384.0),
            ],
            pixel_bbox: PixelBBox::new(64.0, 96.0, 128.0, 288.0),
            visible: true,
        }
    }

    fn sample_bbox() -> LabelObject {
        LabelObject {
            index: 1,
            class_id: 1,
            label_type: LabelType::Bbox,
            normalized_coords: vec![0.5, 0.5, 0.4, 0.6],
            pixel_points: vec![Point::new(192.0, 96.0), Point::new(448.0, 384.0)],
            pixel_bbox: PixelBBox::new(192.0, 96.0, 256.0, 288.0),
            visible: true,
        }
    }

    fn temp_label_dir() -> TempDir {
        let temp = TempDir::new().expect("create temp dir");
        let train = temp.path().join("train");
        fs::create_dir_all(&train).expect("create train dir");
        fs::write(
            train.join("test1.txt"),
            "0 0.1 0.2 0.3 0.2 0.3 0.8 0.2 0.9 0.1 0.8\n2 0.4 0.5 0.6 0.5 0.6 0.7\n",
        )
        .expect("write polygon file");
        fs::write(train.join("test2.txt"), "1 0.5 0.5 0.4 0.6\n")
            .expect("write bbox file");
        fs::write(temp.path().join("empty.txt"), "").expect("write empty file");
        temp
    }

    #[test]
    fn parse_segmentation_polygon() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("train/test1.txt"), 640, 480);
        assert_eq!(labels.len(), 2);
        let object = &labels[0];
        assert_eq!(object.class_id, 0);
        assert_eq!(object.label_type, LabelType::Polygon);
        assert_eq!(object.pixel_points.len(), 5);
        assert_eq!(object.index, 0);
        assert!(object.pixel_bbox.width > 0.0);
        assert!(object.pixel_bbox.height > 0.0);
    }

    #[test]
    fn parse_bounding_box() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("train/test2.txt"), 640, 480);
        assert_eq!(labels.len(), 1);
        let object = &labels[0];
        assert_eq!(object.class_id, 1);
        assert_eq!(object.label_type, LabelType::Bbox);
        approx_eq(object.pixel_bbox.width, 256.0);
        approx_eq(object.pixel_bbox.height, 288.0);
    }

    #[test]
    fn parse_empty_file() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("empty.txt"), 640, 480);
        assert!(labels.is_empty());
    }

    #[test]
    fn parse_missing_file() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("missing.txt"), 640, 480);
        assert!(labels.is_empty());
    }

    #[test]
    fn parse_multi_object_file() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("train/test1.txt"), 640, 480);
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].class_id, 0);
        assert_eq!(labels[1].class_id, 2);
    }

    #[test]
    fn label_indices_are_sequential() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("train/test1.txt"), 640, 480);
        assert_eq!(labels[0].index, 0);
        assert_eq!(labels[1].index, 1);
    }

    #[test]
    fn pixel_points_correctness() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("train/test1.txt"), 640, 480);
        approx_eq(labels[0].pixel_points[0].x, 64.0);
        approx_eq(labels[0].pixel_points[0].y, 96.0);
    }

    #[test]
    fn normalized_coords_preserved() {
        let temp = temp_label_dir();
        let labels = parse_yolo_labels(temp.path().join("train/test1.txt"), 640, 480);
        approx_eq(labels[0].normalized_coords[0], 0.1);
        approx_eq(labels[0].normalized_coords[1], 0.2);
    }

    #[test]
    fn malformed_input_returns_empty_from_file_api() {
        let temp = TempDir::new().expect("create temp dir");
        let path = temp.path().join("bad.txt");
        fs::write(&path, "0 0.1 0.2 0.3\n").expect("write malformed file");
        let labels = parse_yolo_labels(&path, 640, 480);
        assert!(labels.is_empty());
    }

    #[test]
    fn odd_point_count_is_rejected_by_text_parser() {
        let result = parse_yolo_label_text("0 0.1 0.2 0.3 0.4 0.5\n", 640, 480);
        assert!(result.is_err());
    }

    #[test]
    fn roundtrip_polygon() {
        let temp = TempDir::new().expect("create temp dir");
        let out = temp.path().join("out.txt");
        write_yolo_labels(&out, &[sample_polygon()]).expect("write labels");
        let reparsed = parse_yolo_labels(&out, 640, 480);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].class_id, 2);
        assert_eq!(reparsed[0].label_type, LabelType::Polygon);
        approx_eq(reparsed[0].normalized_coords[0], 0.1);
        approx_eq(reparsed[0].normalized_coords[5], 0.8);
    }

    #[test]
    fn roundtrip_bbox() {
        let temp = TempDir::new().expect("create temp dir");
        let out = temp.path().join("out.txt");
        write_yolo_labels(&out, &[sample_bbox()]).expect("write labels");
        let reparsed = parse_yolo_labels(&out, 640, 480);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].class_id, 1);
        assert_eq!(reparsed[0].label_type, LabelType::Bbox);
    }

    #[test]
    fn class_change_roundtrip() {
        let temp = TempDir::new().expect("create temp dir");
        let mut label = sample_polygon();
        label.class_id = 5;
        let out = temp.path().join("out.txt");
        write_yolo_labels(&out, &[label]).expect("write labels");
        let reparsed = parse_yolo_labels(&out, 640, 480);
        assert_eq!(reparsed[0].class_id, 5);
    }

    #[test]
    fn write_creates_parent_dirs() {
        let temp = TempDir::new().expect("create temp dir");
        let out = temp.path().join("sub/dir/out.txt");
        write_yolo_labels(&out, &[]).expect("write labels");
        assert!(out.exists());
        assert_eq!(fs::read_to_string(&out).expect("read file"), "");
    }

    #[test]
    fn write_multiple_labels() {
        let temp = TempDir::new().expect("create temp dir");
        let out = temp.path().join("out.txt");
        write_yolo_labels(&out, &[sample_polygon(), sample_bbox()]).expect("write labels");
        let contents = fs::read_to_string(&out).expect("read file");
        let lines = contents.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("2 "));
        assert!(lines[1].starts_with("1 "));
    }

    #[test]
    fn basic_triangle_from_pixels() {
        let points = [(100.0, 100.0), (200.0, 100.0), (150.0, 200.0)];
        let label = create_label_from_pixels(&points, 640, 480, 2, 0).expect("create label");
        assert_eq!(label.label_type, LabelType::Polygon);
        assert_eq!(label.class_id, 2);
        assert_eq!(label.index, 0);
        assert_eq!(label.pixel_points.len(), 3);
    }

    #[test]
    fn normalized_coords_correct() {
        let points = [(64.0, 96.0), (192.0, 96.0), (192.0, 384.0)];
        let label = create_label_from_pixels(&points, 640, 480, 0, 0).expect("create label");
        approx_eq(label.normalized_coords[0], 0.1);
        approx_eq(label.normalized_coords[1], 0.2);
        approx_eq(label.normalized_coords[5], 0.8);
    }

    #[test]
    fn bounding_box_computed() {
        let points = [(50.0, 100.0), (200.0, 50.0), (150.0, 300.0)];
        let label = create_label_from_pixels(&points, 640, 480, 0, 0).expect("create label");
        approx_eq(label.pixel_bbox.x, 50.0);
        approx_eq(label.pixel_bbox.y, 50.0);
        approx_eq(label.pixel_bbox.width, 150.0);
        approx_eq(label.pixel_bbox.height, 250.0);
    }

    #[test]
    fn too_few_points_raises() {
        let result = create_label_from_pixels(&[(10.0, 10.0), (20.0, 20.0)], 640, 480, 0, 0);
        assert!(result.is_err());
    }

    #[test]
    fn zero_dimensions_raises() {
        let result = create_label_from_pixels(
            &[(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)],
            0,
            480,
            0,
            0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn roundtrip_through_write_and_read() {
        let temp = TempDir::new().expect("create temp dir");
        let points = [(64.0, 96.0), (192.0, 96.0), (192.0, 384.0)];
        let label = create_label_from_pixels(&points, 640, 480, 3, 0).expect("create label");
        let out = temp.path().join("new.txt");
        write_yolo_labels(&out, &[label]).expect("write labels");
        let reparsed = parse_yolo_labels(&out, 640, 480);
        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0].class_id, 3);
        assert_eq!(reparsed[0].label_type, LabelType::Polygon);
        assert_eq!(reparsed[0].pixel_points.len(), 3);
    }

    #[test]
    fn visible_defaults_to_true() {
        let label = create_label_from_pixels(
            &[(10.0, 10.0), (20.0, 20.0), (30.0, 30.0)],
            640,
            480,
            0,
            0,
        )
        .expect("create label");
        assert!(label.visible);
    }

    fn make_labels(count: usize) -> Vec<LabelObject> {
        (0..count)
            .map(|index| LabelObject {
                index,
                class_id: (index % 3) as u32,
                label_type: LabelType::Polygon,
                normalized_coords: vec![0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
                pixel_points: vec![
                    Point::new(64.0, 96.0),
                    Point::new(192.0, 96.0),
                    Point::new(192.0, 384.0),
                ],
                pixel_bbox: PixelBBox::new(64.0, 96.0, 128.0, 288.0),
                visible: true,
            })
            .collect()
    }

    #[test]
    fn delete_middle() {
        let result = delete_label(&make_labels(3), 1);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
        assert_eq!(result[0].class_id, 0);
        assert_eq!(result[1].class_id, 2);
    }

    #[test]
    fn delete_first() {
        let result = delete_label(&make_labels(3), 0);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
    }

    #[test]
    fn delete_last() {
        let result = delete_label(&make_labels(3), 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
    }

    #[test]
    fn delete_nonexistent_index() {
        let result = delete_label(&make_labels(3), 99);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
        assert_eq!(result[2].index, 2);
    }

    #[test]
    fn delete_from_single() {
        let result = delete_label(&make_labels(1), 0);
        assert!(result.is_empty());
    }

    #[test]
    fn delete_from_empty() {
        let result = delete_label(&[], 0);
        assert!(result.is_empty());
    }

    #[test]
    fn delete_roundtrip() {
        let temp = TempDir::new().expect("create temp dir");
        let result = delete_label(&make_labels(3), 1);
        let out = temp.path().join("deleted.txt");
        write_yolo_labels(&out, &result).expect("write labels");
        let reparsed = parse_yolo_labels(&out, 640, 480);
        assert_eq!(reparsed.len(), 2);
        assert_eq!(reparsed[0].class_id, 0);
        assert_eq!(reparsed[1].class_id, 2);
    }
}