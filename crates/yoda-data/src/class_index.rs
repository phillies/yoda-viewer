use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use yoda_core::LabelObject;

use crate::{map_image_to_label_path, FlatIndex, NodeKind};

/// Controls how multiple selected classes are combined during filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FilterMode {
    /// Show images that contain at least one of the selected classes.
    #[default]
    Any,
    /// Show images that contain every one of the selected classes.
    All,
}

/// Sparse map: dataset-relative image path → sorted list of class IDs
/// present in the corresponding label file.
///
/// Persisted to `<label_root>/.yoda_class_index.json` so it survives restarts.
/// The cache file is dot-prefixed and therefore excluded by `scan_dataset_tree`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassIndex {
    /// Key: dataset-relative path (forward slashes, e.g. "train/car25.jpg")
    /// Value: sorted, deduplicated list of class IDs present in that image.
    pub entries: HashMap<String, Vec<u32>>,
}

impl ClassIndex {
    /// Load the persisted index, scan any missing entries, prune stale ones,
    /// and save back if anything changed.
    pub fn load_or_build(image_root: &Path, label_root: &Path, flat_index: &FlatIndex) -> Self {
        let cache_path = label_root.join(".yoda_class_index.json");
        let mut index = Self::load_from_disk(&cache_path).unwrap_or_default();
        let mut dirty = false;

        for node in flat_index.nodes.iter().filter(|n| n.kind == NodeKind::Image) {
            if index.entries.contains_key(&node.path) {
                continue;
            }
            let image_abs = image_root.join(&node.path);
            let Ok(label_abs) = map_image_to_label_path(&image_abs, image_root, label_root) else {
                continue;
            };
            index
                .entries
                .insert(node.path.clone(), extract_class_ids_from_label_file(&label_abs));
            dirty = true;
        }

        // Prune entries for images that no longer exist in the flat index.
        let known_paths: HashSet<&str> = flat_index
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Image)
            .map(|n| n.path.as_str())
            .collect();
        let before = index.entries.len();
        index.entries.retain(|k, _| known_paths.contains(k.as_str()));
        dirty |= index.entries.len() != before;

        if dirty {
            let _ = index.save_to_disk(&cache_path); // best-effort; don't fail startup
        }

        index
    }

    fn load_from_disk(cache_path: &Path) -> Option<Self> {
        let content = fs::read_to_string(cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Persist the index to disk as JSON.
    pub fn save_to_disk(&self, cache_path: &Path) -> std::io::Result<()> {
        let content = serde_json::to_string(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        fs::write(cache_path, content)
    }

    /// Returns the union of all class IDs present anywhere in the dataset.
    pub fn all_class_ids(&self) -> BTreeSet<u32> {
        self.entries.values().flatten().copied().collect()
    }

    /// Returns dataset-relative paths of images that match the given filter.
    pub fn matching_images(&self, required: &BTreeSet<u32>, mode: FilterMode) -> HashSet<&str> {
        if required.is_empty() {
            return self.entries.keys().map(String::as_str).collect();
        }
        self.entries
            .iter()
            .filter(|(_, classes)| match mode {
                FilterMode::Any => classes.iter().any(|id| required.contains(id)),
                FilterMode::All => required.iter().all(|id| classes.contains(id)),
            })
            .map(|(path, _)| path.as_str())
            .collect()
    }
}

/// Extract all unique class IDs from a YOLO label file (best-effort; returns empty on error).
pub fn extract_class_ids_from_label_file(label_path: &Path) -> Vec<u32> {
    let Ok(content) = fs::read_to_string(label_path) else {
        return Vec::new();
    };
    let mut ids: BTreeSet<u32> = BTreeSet::new();
    for line in content.lines() {
        if let Some(first) = line.split_whitespace().next() {
            if let Ok(id) = first.parse::<u32>() {
                ids.insert(id);
            }
        }
    }
    ids.into_iter().collect()
}

/// Extract unique class IDs from an in-memory slice of label objects.
pub fn extract_class_ids(labels: &[LabelObject]) -> Vec<u32> {
    let ids: BTreeSet<u32> = labels.iter().map(|l| l.class_id).collect();
    ids.into_iter().collect()
}
