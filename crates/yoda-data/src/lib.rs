use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use yoda_config::{load_class_map, load_color_map, ConfigError, YoDaSettings};
use yoda_core::{parse_yolo_labels, write_yolo_labels, LabelError, LabelObject};

pub const LAZY_PLACEHOLDER_SUFFIX: &str = "/__lazy__";

const IMAGE_EXTENSIONS: [&str; 5] = [".jpg", ".jpeg", ".png", ".bmp", ".webp"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeNode {
    pub id: String,
    pub label: String,
    pub children: Vec<TreeNode>,
    pub icon: NodeIcon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeIcon {
    Folder,
    Image,
    Placeholder,
}

/// Lightweight node kind used by the flat tree index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Folder,
    Image,
}

/// A single entry in the flat dataset tree index.
///
/// IDs are assigned as sequential indices (0, 1, 2, …) matching the position
/// in the `FlatIndex::nodes` vector, so `nodes[id as usize].id == id` always
/// holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlatNode {
    pub id: u32,
    pub parent_id: Option<u32>,
    pub name: String,
    pub kind: NodeKind,
    /// Dataset-relative path as a UTF-8 string (using `/` separators).
    pub path: String,
}

/// Full dataset tree as a flat, serialisable list.
///
/// The list is ordered depth-first (folders before their contents).
/// Reconstruct the parent→children mapping with [`build_children_map`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlatIndex {
    pub nodes: Arc<[FlatNode]>,
    pub image_count: usize,
}

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Label(#[from] LabelError),
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("image path {image_path:?} is outside image root {image_root:?}")]
    ImageOutsideRoot { image_path: PathBuf, image_root: PathBuf },
}

pub trait DatasetRepository {
    fn list_root_nodes(&self) -> Result<Vec<TreeNode>, RepositoryError>;
    fn expand_directory(&self, path: &Path) -> Result<Vec<TreeNode>, RepositoryError>;
    fn label_path_for_image(&self, image_path: &Path) -> Result<PathBuf, RepositoryError>;
    fn image_dimensions(&self, path: &Path) -> Result<(u32, u32), RepositoryError>;
    fn image_bytes(&self, path: &Path) -> Result<Vec<u8>, RepositoryError>;
    fn load_labels(&self, image_path: &Path) -> Result<Vec<LabelObject>, RepositoryError>;
    fn save_labels(&self, image_path: &Path, labels: &[LabelObject]) -> Result<(), RepositoryError>;
    fn class_map(&self) -> Result<HashMap<u32, String>, RepositoryError>;
    fn color_map(&self) -> Result<HashMap<u32, (u8, u8, u8)>, RepositoryError>;
}

#[derive(Debug, Clone)]
pub struct LocalDatasetRepository {
    settings: YoDaSettings,
}

impl LocalDatasetRepository {
    pub fn new(settings: YoDaSettings) -> Self {
        Self { settings }
    }

    pub fn image_root(&self) -> &Path {
        &self.settings.image_base_path
    }

    pub fn label_root(&self) -> &Path {
        &self.settings.label_base_path
    }

    pub fn label_path_for_image(&self, image_path: &Path) -> Result<PathBuf, RepositoryError> {
        map_image_to_label_path(image_path, self.image_root(), self.label_root())
    }
}

impl DatasetRepository for LocalDatasetRepository {
    fn list_root_nodes(&self) -> Result<Vec<TreeNode>, RepositoryError> {
        Ok(get_file_tree(self.image_root()))
    }

    fn expand_directory(&self, path: &Path) -> Result<Vec<TreeNode>, RepositoryError> {
        Ok(get_dir_children(path))
    }

    fn label_path_for_image(&self, image_path: &Path) -> Result<PathBuf, RepositoryError> {
        LocalDatasetRepository::label_path_for_image(self, image_path)
    }

    fn image_dimensions(&self, path: &Path) -> Result<(u32, u32), RepositoryError> {
        Ok(image::image_dimensions(path)?)
    }

    fn image_bytes(&self, path: &Path) -> Result<Vec<u8>, RepositoryError> {
        Ok(fs::read(path)?)
    }

    fn load_labels(&self, image_path: &Path) -> Result<Vec<LabelObject>, RepositoryError> {
        let label_path = self.label_path_for_image(image_path)?;
        let (width, height) = self.image_dimensions(image_path)?;
        Ok(parse_yolo_labels(label_path, width, height))
    }

    fn save_labels(&self, image_path: &Path, labels: &[LabelObject]) -> Result<(), RepositoryError> {
        let label_path = self.label_path_for_image(image_path)?;
        write_yolo_labels(label_path, labels)?;
        Ok(())
    }

    fn class_map(&self) -> Result<HashMap<u32, String>, RepositoryError> {
        Ok(load_class_map(self.settings.class_info.as_deref()))
    }

    fn color_map(&self) -> Result<HashMap<u32, (u8, u8, u8)>, RepositoryError> {
        let (color_map, _) = load_color_map(self.settings.color_map.as_deref())?;
        Ok(color_map)
    }
}

pub fn get_files(base_dir: &Path) -> Vec<PathBuf> {
    if !base_dir.exists() {
        return Vec::new();
    }

    walk_dir(base_dir)
        .into_iter()
        .filter(|path| path.is_file())
        .collect()
}

pub fn get_dir_children(path: &Path) -> Vec<TreeNode> {
    if !path.exists() || !path.is_dir() {
        return Vec::new();
    }

    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    // Pre-compute the sort key once per entry to avoid O(n log n) allocations
    // inside the comparator.
    let mut items_with_keys: Vec<(PathBuf, bool, String)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| !name.starts_with('.'))
                .unwrap_or(false)
        })
        .map(|path| {
            let is_dir = path.is_dir();
            let sort_key = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            (path, is_dir, sort_key)
        })
        .collect();

    items_with_keys.sort_by(|(_, a_is_dir, a_key), (_, b_is_dir, b_key)| {
        (!a_is_dir, a_key).cmp(&(!b_is_dir, b_key))
    });

    items_with_keys
        .into_iter()
        .filter_map(|(path, _, _)| tree_node_for_path(&path))
        .collect()
}

pub fn get_file_tree(path: &Path) -> Vec<TreeNode> {
    get_dir_children(path)
}

pub fn map_image_to_label_path(
    image_path: &Path,
    image_root: &Path,
    label_root: &Path,
) -> Result<PathBuf, RepositoryError> {
    let relative = image_path
        .strip_prefix(image_root)
        .map_err(|_| RepositoryError::ImageOutsideRoot {
            image_path: image_path.to_path_buf(),
            image_root: image_root.to_path_buf(),
        })?;

    Ok(label_root.join(relative).with_extension("txt"))
}

fn walk_dir(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut results = Vec::new();
    for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            results.extend(walk_dir(&entry_path));
        } else {
            results.push(entry_path);
        }
    }
    results
}

fn tree_node_for_path(path: &Path) -> Option<TreeNode> {
    let label = path.file_name()?.to_string_lossy().to_string();
    let id = path.to_string_lossy().to_string();

    if path.is_dir() {
        return Some(TreeNode {
            id,
            label,
            children: vec![TreeNode {
                id: format!("{}{}", path.to_string_lossy(), LAZY_PLACEHOLDER_SUFFIX),
                label: String::from("..."),
                children: Vec::new(),
                icon: NodeIcon::Placeholder,
            }],
            icon: NodeIcon::Folder,
        });
    }

    let extension = path.extension()?.to_string_lossy().to_lowercase();
    let extension = format!(".{extension}");
    if !IMAGE_EXTENSIONS.contains(&extension.as_str()) {
        return None;
    }

    Some(TreeNode {
        id,
        label,
        children: Vec::new(),
        icon: NodeIcon::Image,
    })
}

// ─── Flat index scan ─────────────────────────────────────────────────────────

/// Walk *root* synchronously and return a [`FlatIndex`] of every folder and
/// image file found, excluding hidden entries (names starting with `.`).
///
/// Node IDs are assigned as sequential indices so that `nodes[id as usize].id
/// == id` always holds.  The walk is depth-first, folders first within each
/// directory level.
pub fn scan_dataset_tree(root: &Path) -> FlatIndex {
    let mut nodes: Vec<FlatNode> = Vec::new();
    let mut image_count: usize = 0;
    if root.is_dir() {
        scan_dir_flat(root, root, None, &mut nodes, &mut image_count);
    }
    FlatIndex {
        nodes: nodes.into(),
        image_count,
    }
}

fn scan_dir_flat(
    root: &Path,
    path: &Path,
    parent_id: Option<u32>,
    nodes: &mut Vec<FlatNode>,
    image_count: &mut usize,
) {
    let Ok(entries) = fs::read_dir(path) else { return };

    let mut items_with_keys: Vec<(PathBuf, bool, String)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| !n.starts_with('.'))
                .unwrap_or(false)
        })
        .map(|p| {
            let is_dir = p.is_dir();
            let sort_key = p
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            (p, is_dir, sort_key)
        })
        .collect();

    items_with_keys.sort_by(|(_, a_is_dir, a_key), (_, b_is_dir, b_key)| {
        (!a_is_dir, a_key).cmp(&(!b_is_dir, b_key))
    });

    for (entry_path, is_dir, _) in items_with_keys {
        let Some(name) = entry_path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };

        if is_dir {
            let id = nodes.len() as u32;
            nodes.push(FlatNode {
                id,
                parent_id,
                name,
                kind: NodeKind::Folder,
                path: dataset_relative_path(root, &entry_path),
            });
            scan_dir_flat(root, &entry_path, Some(id), nodes, image_count);
        } else {
            let ext = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{}", e.to_lowercase()))
                .unwrap_or_default();
            if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                let id = nodes.len() as u32;
                nodes.push(FlatNode {
                    id,
                    parent_id,
                    name,
                    kind: NodeKind::Image,
                    path: dataset_relative_path(root, &entry_path),
                });
                *image_count += 1;
            }
        }
    }
}

fn dataset_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod scan_tests {
    use std::fs;
    use tempfile::TempDir;
    use super::{scan_dataset_tree, NodeKind};

    fn make_dataset(layout: &[(&str, &[u8])]) -> TempDir {
        let tmp = TempDir::new().expect("tempdir");
        for (rel, content) in layout {
            let dest = tmp.path().join(rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).expect("mkdir");
            }
            fs::write(&dest, content).expect("write");
        }
        tmp
    }

    #[test]
    fn scan_counts_images_and_skips_hidden() {
        let tmp = make_dataset(&[
            ("train/a.jpg", b""),
            ("train/b.png", b""),
            ("val/c.jpg", b""),
            ("val/.hidden.jpg", b""),
            ("val/notes.txt", b""),
        ]);
        let idx = scan_dataset_tree(tmp.path());
        assert_eq!(idx.image_count, 3, "should count 3 images");
        assert!(
            idx.nodes.iter().all(|n| !n.name.starts_with('.')),
            "hidden entries must be excluded"
        );
    }

    #[test]
    fn scan_id_matches_index() {
        let tmp = make_dataset(&[
            ("train/a.jpg", b""),
            ("train/b.jpg", b""),
        ]);
        let idx = scan_dataset_tree(tmp.path());
        for (i, node) in idx.nodes.iter().enumerate() {
            assert_eq!(node.id as usize, i, "node.id must equal its position");
        }
    }

    #[test]
    fn scan_folders_before_images() {
        let tmp = make_dataset(&[
            ("b_folder/x.jpg", b""),
            ("a.jpg", b""),
        ]);
        let idx = scan_dataset_tree(tmp.path());
        // "b_folder" (a Folder) should appear before "a.jpg" (an Image)
        let folder_pos = idx.nodes.iter().position(|n| n.kind == NodeKind::Folder).expect("folder");
        let image_pos  = idx.nodes.iter().position(|n| n.kind == NodeKind::Image && n.parent_id.is_none()).expect("root image");
        assert!(folder_pos < image_pos, "folders must sort before images");
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;
    use yoda_config::YoDaSettings;

    use super::{
        get_dir_children, get_file_tree, get_files, map_image_to_label_path,
        DatasetRepository, LocalDatasetRepository, NodeIcon, LAZY_PLACEHOLDER_SUFFIX,
    };

    fn sample_dataset() -> TempDir {
        let temp = TempDir::new().expect("create temp dir");
        let image_dir = temp.path().join("images/train");
        let label_dir = temp.path().join("labels/train");
        fs::create_dir_all(&image_dir).expect("create image dir");
        fs::create_dir_all(&label_dir).expect("create label dir");

        let image = image::RgbImage::from_pixel(640, 480, image::Rgb([128, 128, 128]));
        image.save(image_dir.join("test1.jpg")).expect("save jpg");
        image.save(image_dir.join("test2.png")).expect("save png");
        fs::write(
            label_dir.join("test1.txt"),
            "0 0.1 0.2 0.3 0.2 0.3 0.8 0.1 0.8 0.05 0.5\n2 0.5 0.5 0.7 0.5 0.7 0.9 0.5 0.9\n",
        )
        .expect("write labels");
        fs::write(label_dir.join("test2.txt"), "1 0.5 0.5 0.4 0.6\n").expect("write labels");

        temp
    }

    #[test]
    fn finds_all_files() {
        let temp = sample_dataset();
        let files = get_files(&temp.path().join("images"));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn empty_dir() {
        let temp = TempDir::new().expect("create temp dir");
        let empty = temp.path().join("empty");
        fs::create_dir_all(&empty).expect("create empty dir");
        assert!(get_files(&empty).is_empty());
    }

    #[test]
    fn nested_structure() {
        let temp = TempDir::new().expect("create temp dir");
        let nested = temp.path().join("a/b/c");
        fs::create_dir_all(&nested).expect("create nested dir");
        fs::write(nested.join("deep.jpg"), b"x").expect("write deep file");
        fs::write(temp.path().join("a/top.txt"), b"y").expect("write txt");
        let files = get_files(&temp.path().join("a"));
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn tree_structure() {
        let temp = sample_dataset();
        let tree = get_file_tree(&temp.path().join("images"));
        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].label, "train");
        assert_eq!(tree[0].children.len(), 1);
        assert!(tree[0].children[0].id.ends_with(LAZY_PLACEHOLDER_SUFFIX));
    }

    #[test]
    fn skips_non_image_files() {
        let temp = TempDir::new().expect("create temp dir");
        let dir = temp.path().join("dir");
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join("data.txt"), "hello").expect("write text file");
        fs::write(dir.join("photo.jpg"), b"\xff\xd8\xff\xe0").expect("write image");
        let tree = get_file_tree(&dir);
        let labels = tree.iter().map(|node| node.label.as_str()).collect::<Vec<_>>();
        assert!(labels.contains(&"photo.jpg"));
        assert!(!labels.contains(&"data.txt"));
    }

    #[test]
    fn skips_hidden_files() {
        let temp = TempDir::new().expect("create temp dir");
        let dir = temp.path().join("dir");
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join(".hidden.jpg"), b"x").expect("write hidden image");
        fs::write(dir.join("visible.png"), b"x").expect("write visible image");
        let tree = get_file_tree(&dir);
        let labels = tree.iter().map(|node| node.label.as_str()).collect::<Vec<_>>();
        assert!(!labels.contains(&".hidden.jpg"));
        assert!(labels.contains(&"visible.png"));
    }

    #[test]
    fn nonexistent_dir() {
        let temp = TempDir::new().expect("create temp dir");
        assert!(get_file_tree(&temp.path().join("nope")).is_empty());
    }

    #[test]
    fn directories_first() {
        let temp = TempDir::new().expect("create temp dir");
        let dir = temp.path().join("root");
        fs::create_dir_all(dir.join("a_folder")).expect("create folder");
        fs::write(dir.join("z_image.jpg"), b"x").expect("write image");
        let tree = get_file_tree(&dir);
        assert_eq!(tree[0].label, "a_folder");
        assert_eq!(tree[0].icon, NodeIcon::Folder);
        assert_eq!(tree[1].label, "z_image.jpg");
    }

    #[test]
    fn lists_images_in_dir() {
        let temp = sample_dataset();
        let children = get_dir_children(&temp.path().join("images/train"));
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn dir_gets_lazy_placeholder() {
        let temp = TempDir::new().expect("create temp dir");
        let child = temp.path().join("parent/subdir");
        fs::create_dir_all(&child).expect("create subdir");
        fs::write(child.join("img.jpg"), b"x").expect("write image");
        let nodes = get_dir_children(&temp.path().join("parent"));
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label, "subdir");
        assert_eq!(nodes[0].children.len(), 1);
        assert!(nodes[0].children[0].id.ends_with(LAZY_PLACEHOLDER_SUFFIX));
    }

    #[test]
    fn nonexistent_dir_returns_empty() {
        let temp = TempDir::new().expect("create temp dir");
        assert!(get_dir_children(&temp.path().join("missing")).is_empty());
    }

    #[test]
    fn skips_hidden_and_non_image() {
        let temp = TempDir::new().expect("create temp dir");
        let dir = temp.path().join("d");
        fs::create_dir_all(&dir).expect("create dir");
        fs::write(dir.join(".hidden.jpg"), b"x").expect("write hidden image");
        fs::write(dir.join("notes.txt"), b"hi").expect("write txt");
        fs::write(dir.join("photo.png"), b"x").expect("write image");
        let nodes = get_dir_children(&dir);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label, "photo.png");
    }

    #[test]
    fn all_image_extensions_included() {
        let temp = TempDir::new().expect("create temp dir");
        let dir = temp.path().join("d");
        fs::create_dir_all(&dir).expect("create dir");
        for extension in [".jpg", ".jpeg", ".png", ".bmp", ".webp"] {
            let file_name = format!("img{extension}");
            fs::write(dir.join(&file_name), b"x").expect("write image");
        }
        let nodes = get_dir_children(&dir);
        assert_eq!(nodes.len(), 5);
    }

    #[test]
    fn repository_resolves_matching_label_path() {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let repository = LocalDatasetRepository::new(settings);
        let label_path = repository
            .label_path_for_image(&temp.path().join("images/train/test1.jpg"))
            .expect("resolve label path");
        assert_eq!(label_path, temp.path().join("labels/train/test1.txt"));
    }

    #[test]
    fn repository_loads_labels_for_image() {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let repository = LocalDatasetRepository::new(settings);
        let labels = repository
            .load_labels(&temp.path().join("images/train/test1.jpg"))
            .expect("load labels");
        assert_eq!(labels.len(), 2);
    }

    #[test]
    fn repository_saves_labels_to_mirrored_label_path() {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let repository = LocalDatasetRepository::new(settings);
        let image_path = temp.path().join("images/train/test2.png");
        let mut labels = repository.load_labels(&image_path).expect("load labels");
        labels[0].class_id = 9;
        repository.save_labels(&image_path, &labels).expect("save labels");
        let written = fs::read_to_string(temp.path().join("labels/train/test2.txt"))
            .expect("read saved labels");
        assert!(written.starts_with("9 "));
    }

    #[test]
    fn example_data_image_path_maps_to_expected_label_path() {
        let mapped = map_image_to_label_path(
            Path::new("example_data/images/test/car4_jpg.rf.8978131a7b03be689c244641e42e1307.jpg"),
            Path::new("example_data/images"),
            Path::new("example_data/labels"),
        )
        .expect("map example_data path");
        assert_eq!(
            mapped,
            Path::new("example_data/labels/test/car4_jpg.rf.8978131a7b03be689c244641e42e1307.txt")
        );
    }
}