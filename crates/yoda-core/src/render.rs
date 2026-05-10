use std::collections::HashMap;

use crate::{LabelObject, LabelType, PixelBBox, Point};

const LABEL_FONT_FAMILY: &str = "'JetBrains Mono', 'Cascadia Code', 'Consolas', monospace";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOptions {
    pub show_bbox: bool,
    pub show_segmask: bool,
    pub show_class_id: bool,
    pub show_class_name: bool,
    pub selected_index: Option<usize>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            show_bbox: false,
            show_segmask: true,
            show_class_id: false,
            show_class_name: false,
            selected_index: None,
        }
    }
}

pub fn default_color_for_class(class_id: u32) -> (u8, u8, u8) {
    hsv_to_rgb(((class_id as f32 * 137.5) % 360.0) / 360.0, 1.0, 1.0)
}

pub fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len().saturating_sub(1);

    for current in 0..polygon.len() {
        let current_point = polygon[current];
        let previous_point = polygon[previous];
        let intersects = ((current_point.y > point.y) != (previous_point.y > point.y))
            && (point.x
                < (previous_point.x - current_point.x)
                    * (point.y - current_point.y)
                    / (previous_point.y - current_point.y)
                    + current_point.x);

        if intersects {
            inside = !inside;
        }

        previous = current;
    }

    inside
}

pub fn bbox_contains(bbox: PixelBBox, point: Point) -> bool {
    bbox.x <= point.x
        && point.x <= bbox.x + bbox.width
        && bbox.y <= point.y
        && point.y <= bbox.y + bbox.height
}

pub fn hit_test_labels(labels: &[LabelObject], point: Point) -> Option<usize> {
    labels.iter().rev().find_map(|label| {
        if !label.visible {
            return None;
        }

        match label.label_type {
            LabelType::Polygon if point_in_polygon(point, &label.pixel_points) => Some(label.index),
            LabelType::Bbox if bbox_contains(label.pixel_bbox, point) => Some(label.index),
            _ => None,
        }
    })
}

pub fn render_labels_to_svg(
    labels: &[LabelObject],
    color_map: Option<&HashMap<u32, (u8, u8, u8)>>,
    class_map: Option<&HashMap<u32, String>>,
    options: &RenderOptions,
) -> String {
    let mut elements = Vec::new();

    for label in labels {
        if !label.visible {
            continue;
        }

        let (red, green, blue) = color_map
            .and_then(|map| map.get(&label.class_id).copied())
            .unwrap_or_else(|| default_color_for_class(label.class_id));
        let color = format!("rgb({red},{green},{blue})");
        let selected = options.selected_index == Some(label.index);

        if options.show_segmask && label.label_type == LabelType::Polygon {
            elements.push(render_segmask(label, &color, selected));
        }

        if options.show_bbox {
            elements.push(render_bbox(label, &color));
        }

        if options.show_class_id || options.show_class_name {
            elements.push(render_text_label(label, class_map, options));
        }
    }

    elements.join("")
}

fn render_segmask(label: &LabelObject, color: &str, selected: bool) -> String {
    let points = label
        .pixel_points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(" ");

    if selected {
        return format!(
            "<polygon points=\"{points}\" fill=\"{color}\" stroke=\"white\" fill-opacity=\"0.5\" stroke-width=\"3\" stroke-dasharray=\"8,3\" style=\"animation: marchingAnts 0.6s linear infinite;\" />"
        );
    }

    format!(
        "<polygon points=\"{points}\" fill=\"{color}\" stroke=\"{color}\" fill-opacity=\"0.3\" stroke-width=\"2\" />"
    )
}

fn render_bbox(label: &LabelObject, color: &str) -> String {
    let bbox = label.pixel_bbox;
    if label.label_type == LabelType::Bbox {
        return format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{color}\" stroke=\"{color}\" fill-opacity=\"0.2\" stroke-width=\"2\" />",
            bbox.x, bbox.y, bbox.width, bbox.height
        );
    }

    format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"2\" stroke-dasharray=\"6,3\" />",
        bbox.x, bbox.y, bbox.width, bbox.height
    )
}

fn render_text_label(
    label: &LabelObject,
    class_map: Option<&HashMap<u32, String>>,
    options: &RenderOptions,
) -> String {
    let mut text_parts = Vec::new();
    if options.show_class_id {
        text_parts.push(label.class_id.to_string());
    }
    if options.show_class_name {
        let class_name = class_map
            .and_then(|map| map.get(&label.class_id))
            .cloned()
            .unwrap_or_else(|| format!("cls_{}", label.class_id));
        text_parts.push(class_name);
    }

    let text = text_parts.join(" ");
    let text_width = ((text.chars().count() as f32) * 6.0).max(20.0) + 8.0;
    let text_height = 16.0_f32;
    let x = label.pixel_bbox.x;
    let y = label.pixel_bbox.y;

    format!(
        "<rect x=\"{x}\" y=\"{}\" width=\"{text_width}\" height=\"{text_height}\" fill=\"rgba(0,0,0,0.7)\" rx=\"3\" /><text x=\"{}\" y=\"{}\" fill=\"white\" font-size=\"10\" font-family=\"{}\">{text}</text>",
        y - text_height,
        x + 4.0,
        y - 4.0,
        LABEL_FONT_FAMILY,
    )
}

fn hsv_to_rgb(hue: f32, saturation: f32, value: f32) -> (u8, u8, u8) {
    let sector = (hue * 6.0).floor();
    let fraction = hue * 6.0 - sector;
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - fraction * saturation);
    let t = value * (1.0 - (1.0 - fraction) * saturation);

    let (red, green, blue) = match sector as i32 % 6 {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };

    (
        (red * 255.0).round() as u8,
        (green * 255.0).round() as u8,
        (blue * 255.0).round() as u8,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use insta::assert_snapshot;

    use crate::{LabelObject, LabelType, PixelBBox, Point};

    use super::{
        bbox_contains, hit_test_labels, point_in_polygon, render_labels_to_svg,
        RenderOptions, LABEL_FONT_FAMILY,
    };

    fn sample_labels() -> Vec<LabelObject> {
        vec![
            LabelObject {
                index: 0,
                class_id: 0,
                label_type: LabelType::Polygon,
                normalized_coords: vec![0.1, 0.2, 0.3, 0.2, 0.3, 0.8],
                pixel_points: vec![
                    Point::new(64.0, 96.0),
                    Point::new(192.0, 96.0),
                    Point::new(192.0, 384.0),
                ],
                pixel_bbox: PixelBBox::new(64.0, 96.0, 128.0, 288.0),
                visible: true,
            },
            LabelObject {
                index: 1,
                class_id: 1,
                label_type: LabelType::Bbox,
                normalized_coords: vec![0.5, 0.5, 0.4, 0.6],
                pixel_points: vec![Point::new(192.0, 96.0), Point::new(448.0, 384.0)],
                pixel_bbox: PixelBBox::new(192.0, 96.0, 256.0, 288.0),
                visible: true,
            },
        ]
    }

    fn color_map() -> HashMap<u32, (u8, u8, u8)> {
        HashMap::from([(0, (255, 0, 0)), (1, (0, 255, 0))])
    }

    #[test]
    fn segmask_only() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions::default(),
        );
        assert!(svg.contains("<polygon"));
        assert!(!svg.contains("<rect"));
    }

    #[test]
    fn bbox_only() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions {
                show_bbox: true,
                show_segmask: false,
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("<rect"));
        assert!(svg.contains("stroke-dasharray"));
        assert!(!svg.contains("<polygon"));
    }

    #[test]
    fn both_modes() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions {
                show_bbox: true,
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("<polygon"));
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn nothing_shown() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions {
                show_bbox: false,
                show_segmask: false,
                show_class_id: false,
                show_class_name: false,
                selected_index: None,
            },
        );
        assert!(svg.is_empty());
    }

    #[test]
    fn class_id_text() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions {
                show_segmask: false,
                show_class_id: true,
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("<text"));
        assert!(svg.contains(">0</text>"));
        assert!(svg.contains(">1</text>"));
    }

    #[test]
    fn class_name_text() {
        let class_map = HashMap::from([
            (0, String::from("bumper")),
            (1, String::from("wheel")),
        ]);
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            Some(&class_map),
            &RenderOptions {
                show_segmask: false,
                show_class_name: true,
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("bumper"));
        assert!(svg.contains("wheel"));
    }

    #[test]
    fn uses_correct_colors() {
        let custom_colors = HashMap::from([(0, (10, 20, 30)), (1, (40, 50, 60))]);
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&custom_colors),
            None,
            &RenderOptions {
                show_bbox: true,
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("rgb(10,20,30)"));
        assert!(svg.contains("rgb(40,50,60)"));
    }

    #[test]
    fn invisible_labels_skipped() {
        let mut labels = sample_labels();
        labels[0].visible = false;
        let svg = render_labels_to_svg(
            &labels,
            Some(&color_map()),
            None,
            &RenderOptions {
                show_bbox: true,
                ..RenderOptions::default()
            },
        );
        assert!(!svg.contains("rgb(255,0,0)"));
        assert!(svg.contains("rgb(0,255,0)"));
    }

    #[test]
    fn empty_labels() {
        let svg = render_labels_to_svg(&[], None, None, &RenderOptions::default());
        assert!(svg.is_empty());
    }

    #[test]
    fn class_id_and_name_combined() {
        let class_map = HashMap::from([
            (0, String::from("bumper")),
            (1, String::from("wheel")),
        ]);
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            Some(&class_map),
            &RenderOptions {
                show_segmask: false,
                show_class_id: true,
                show_class_name: true,
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("0 bumper"));
        assert!(svg.contains("1 wheel"));
        assert!(svg.contains("width=\"56\""));
        assert!(svg.contains("width=\"50\""));
        assert!(svg.contains(LABEL_FONT_FAMILY));
    }

    #[test]
    fn selected_index_highlights_polygon() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions {
                selected_index: Some(0),
                ..RenderOptions::default()
            },
        );
        assert!(svg.contains("stroke=\"white\""));
        assert!(svg.contains("stroke-dasharray"));
    }

    #[test]
    fn selected_index_none_no_highlight() {
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            None,
            &RenderOptions::default(),
        );
        assert!(!svg.contains("stroke=\"white\""));
        assert!(!svg.contains("animation: marchingAnts"));
    }

    #[test]
    fn point_in_polygon_matches_python_behavior() {
        let polygon = vec![
            Point::new(64.0, 96.0),
            Point::new(192.0, 96.0),
            Point::new(192.0, 384.0),
        ];
        assert!(point_in_polygon(Point::new(150.0, 150.0), &polygon));
        assert!(!point_in_polygon(Point::new(10.0, 10.0), &polygon));
    }

    #[test]
    fn bbox_contains_matches_python_behavior() {
        let bbox = PixelBBox::new(192.0, 96.0, 256.0, 288.0);
        assert!(bbox_contains(bbox, Point::new(200.0, 100.0)));
        assert!(!bbox_contains(bbox, Point::new(10.0, 10.0)));
    }

    #[test]
    fn hit_test_uses_reverse_paint_order() {
        let labels = sample_labels();
        let hit = hit_test_labels(&labels, Point::new(200.0, 100.0));
        assert_eq!(hit, Some(1));
    }

    #[test]
    fn overlay_snapshot() {
        let class_map = HashMap::from([
            (0, String::from("bumper")),
            (1, String::from("wheel")),
        ]);
        let svg = render_labels_to_svg(
            &sample_labels(),
            Some(&color_map()),
            Some(&class_map),
            &RenderOptions {
                show_bbox: true,
                show_class_id: true,
                show_class_name: true,
                selected_index: Some(0),
                ..RenderOptions::default()
            },
        );
        assert_snapshot!("overlay_render", svg);
    }
}