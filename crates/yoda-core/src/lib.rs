mod label;

pub use crate::label::{
    create_label_from_pixels, delete_label, parse_yolo_labels, write_yolo_labels,
    LabelObject, LabelType, PixelBBox, Point,
};