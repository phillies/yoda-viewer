use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use yoda_core::LabelObject;

use crate::{FlatIndex, NodeKind, map_image_to_label_path};

/// Controls how multiple selected classes are combined during filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FilterMode {
    /// Show images that contain at least one of the selected classes.
    #[default]
    Any,
    /// Show images that contain every one of the selected classes.
    All,
}

/// File name of the SQLite cache stored in the label root.
pub const INDEX_DB_FILE: &str = ".yoda_index.sqlite";

/// Outcome of an index refresh.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshStats {
    pub total: usize,
    pub rescanned: usize,
    pub removed: usize,
}

/// Dataset statistics: image counts overall and per class.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetStats {
    pub images: usize,
    pub unlabeled: usize,
    /// (class id, number of images containing that class), sorted by class id.
    pub images_per_class: Vec<(u32, usize)>,
}

/// Sparse map: dataset-relative image path → sorted list of class IDs
/// present in the corresponding label file.
///
/// Persisted to `<label_root>/.yoda_index.sqlite`, keyed by label file
/// mtime/size so that refreshes only re-read label files that changed.
/// The cache file is dot-prefixed and therefore excluded by `scan_dataset_tree`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassIndex {
    /// Key: dataset-relative path (forward slashes, e.g. "train/car25.jpg")
    /// Value: sorted, deduplicated list of class IDs present in that image.
    pub entries: HashMap<String, Vec<u32>>,
    #[serde(skip)]
    db_path: Option<PathBuf>,
}

/// (mtime in ns, size in bytes) of a label file; (0, 0) when it does not exist.
type LabelSignature = (i64, i64);

fn label_signature(label_path: &Path) -> LabelSignature {
    let Ok(meta) = fs::metadata(label_path) else { return (0, 0) };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0);
    (mtime, meta.len() as i64)
}

fn encode_classes(ids: &[u32]) -> String {
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

fn decode_classes(text: &str) -> Vec<u32> {
    text.split(',').filter_map(|part| part.parse().ok()).collect()
}

fn open_db(path: &Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = DELETE;
         CREATE TABLE IF NOT EXISTS images (
             path TEXT PRIMARY KEY,
             mtime_ns INTEGER NOT NULL,
             size INTEGER NOT NULL,
             classes TEXT NOT NULL
         );",
    )?;
    Ok(conn)
}

fn persist(
    conn: &mut Connection,
    scanned: &[(String, Vec<u32>)],
    signatures: &HashMap<String, LabelSignature>,
    stale: &[&String],
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    {
        let mut upsert = tx.prepare(
            "INSERT OR REPLACE INTO images (path, mtime_ns, size, classes) VALUES (?1, ?2, ?3, ?4)",
        )?;
        for (path, classes) in scanned {
            let (mtime, size) = signatures.get(path).copied().unwrap_or((0, 0));
            upsert.execute(rusqlite::params![
                path,
                mtime,
                size,
                encode_classes(classes)
            ])?;
        }
        let mut delete = tx.prepare("DELETE FROM images WHERE path = ?1")?;
        for path in stale {
            delete.execute([path.as_str()])?;
        }
    }
    tx.commit()
}

impl ClassIndex {
    /// Open (or create) the SQLite cache and bring it up to date.
    ///
    /// Persistence is best-effort: if the cache cannot be opened or written the
    /// index is still built in memory so startup never fails.
    pub fn load_or_build(
        image_root: &Path,
        label_root: &Path,
        flat_index: &FlatIndex,
    ) -> Self {
        let mut index = Self {
            entries: HashMap::new(),
            db_path: Some(label_root.join(INDEX_DB_FILE)),
        };
        index.refresh(image_root, label_root, flat_index);
        index
    }

    /// Re-synchronise with the label files on disk. Only labels whose
    /// mtime/size differ from the cached signature are re-read (in parallel).
    pub fn refresh(
        &mut self,
        image_root: &Path,
        label_root: &Path,
        flat_index: &FlatIndex,
    ) -> RefreshStats {
        let conn = self.db_path.as_deref().and_then(|p| open_db(p).ok());

        let mut cached: HashMap<String, (LabelSignature, Vec<u32>)> = HashMap::new();
        if let Some(conn) = &conn {
            if let Ok(mut stmt) =
                conn.prepare("SELECT path, mtime_ns, size, classes FROM images")
            {
                if let Ok(rows) = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        (row.get::<_, i64>(1)?, row.get::<_, i64>(2)?),
                        row.get::<_, String>(3)?,
                    ))
                }) {
                    for (path, sig, classes) in rows.flatten() {
                        cached.insert(path, (sig, decode_classes(&classes)));
                    }
                }
            }
        }

        let mut entries: HashMap<String, Vec<u32>> = HashMap::new();
        let mut signatures: HashMap<String, LabelSignature> = HashMap::new();
        let mut todo: Vec<(String, PathBuf, LabelSignature)> = Vec::new();
        for node in flat_index.nodes.iter().filter(|n| n.kind == NodeKind::Image) {
            let image_abs = image_root.join(&node.path);
            let Ok(label_abs) =
                map_image_to_label_path(&image_abs, image_root, label_root)
            else {
                continue;
            };
            let sig = label_signature(&label_abs);
            match cached.get(&node.path) {
                Some((cached_sig, classes)) if *cached_sig == sig => {
                    entries.insert(node.path.clone(), classes.clone());
                }
                _ => todo.push((node.path.clone(), label_abs, sig)),
            }
            signatures.insert(node.path.clone(), sig);
        }

        let rescanned = todo.len();
        let workers =
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
        let chunk_size = todo.len().div_ceil(workers).max(1);
        let scanned: Vec<(String, Vec<u32>)> = std::thread::scope(|scope| {
            let handles: Vec<_> = todo
                .chunks(chunk_size)
                .map(|chunk| {
                    scope.spawn(move || {
                        chunk
                            .iter()
                            .map(|(path, label, _)| {
                                (path.clone(), extract_class_ids_from_label_file(label))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).flatten().collect()
        });
        let stale: Vec<&String> = cached
            .keys()
            .filter(|path| !signatures.contains_key(path.as_str()))
            .collect();
        let removed = stale.len();

        if let Some(mut conn) = conn {
            if rescanned > 0 || removed > 0 {
                let _ = persist(&mut conn, &scanned, &signatures, &stale);
            }
        }

        for (path, classes) in scanned {
            entries.insert(path, classes);
        }
        let total = entries.len();
        self.entries = entries;
        RefreshStats { total, rescanned, removed }
    }

    /// Record the classes of a single image after its labels were saved.
    pub fn update_entry(
        &mut self,
        image_rel: &str,
        label_path: &Path,
        class_ids: Vec<u32>,
    ) {
        if let Some(conn) = self.db_path.as_deref().and_then(|p| open_db(p).ok()) {
            let (mtime, size) = label_signature(label_path);
            let _ = conn.execute(
                "INSERT OR REPLACE INTO images (path, mtime_ns, size, classes) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![image_rel, mtime, size, encode_classes(&class_ids)],
            );
        }
        self.entries.insert(image_rel.to_owned(), class_ids);
    }

    /// Aggregate dataset statistics derived from the index.
    pub fn stats(&self) -> DatasetStats {
        let mut per_class: std::collections::BTreeMap<u32, usize> = Default::default();
        let mut unlabeled = 0;
        for classes in self.entries.values() {
            if classes.is_empty() {
                unlabeled += 1;
            }
            for id in classes {
                *per_class.entry(*id).or_default() += 1;
            }
        }
        DatasetStats {
            images: self.entries.len(),
            unlabeled,
            images_per_class: per_class.into_iter().collect(),
        }
    }

    /// Returns the union of all class IDs present anywhere in the dataset.
    pub fn all_class_ids(&self) -> BTreeSet<u32> {
        self.entries.values().flatten().copied().collect()
    }

    /// Returns dataset-relative paths of images that match the given filter.
    pub fn matching_images(
        &self,
        required: &BTreeSet<u32>,
        mode: FilterMode,
    ) -> HashSet<&str> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan_dataset_tree;

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().expect("tempdir");
        let images = temp.path().join("images");
        let labels = temp.path().join("labels");
        fs::create_dir_all(&images).expect("images dir");
        fs::create_dir_all(&labels).expect("labels dir");
        for name in ["a", "b"] {
            fs::write(images.join(format!("{name}.jpg")), b"x").expect("image");
        }
        fs::write(labels.join("a.txt"), "0 0.5 0.5 0.1 0.1\n2 0.5 0.5 0.1 0.1\n")
            .expect("label");
        (temp, images, labels)
    }

    #[test]
    fn builds_persists_and_reuses_cache() {
        let (_temp, images, labels) = setup();
        let flat = scan_dataset_tree(&images);
        let index = ClassIndex::load_or_build(&images, &labels, &flat);
        assert_eq!(index.entries.get("a.jpg"), Some(&vec![0, 2]));
        assert_eq!(index.entries.get("b.jpg"), Some(&Vec::new()));
        assert!(labels.join(INDEX_DB_FILE).exists());

        let mut again = ClassIndex::load_or_build(&images, &labels, &flat);
        let stats = again.refresh(&images, &labels, &flat);
        assert_eq!(stats.rescanned, 0);
        assert_eq!(again.entries, index.entries);
    }

    #[test]
    fn refresh_detects_external_edits_and_removals() {
        let (_temp, images, labels) = setup();
        let flat = scan_dataset_tree(&images);
        let mut index = ClassIndex::load_or_build(&images, &labels, &flat);

        fs::write(labels.join("b.txt"), "5 0.5 0.5 0.1 0.1\n").expect("label");
        let stats = index.refresh(&images, &labels, &flat);
        assert_eq!(stats.rescanned, 1);
        assert_eq!(index.entries.get("b.jpg"), Some(&vec![5]));

        fs::remove_file(images.join("a.jpg")).expect("remove");
        let flat = scan_dataset_tree(&images);
        let stats = index.refresh(&images, &labels, &flat);
        assert_eq!(stats.removed, 1);
        assert!(!index.entries.contains_key("a.jpg"));
    }

    #[test]
    fn stats_count_images_per_class() {
        let (_temp, images, labels) = setup();
        let flat = scan_dataset_tree(&images);
        let stats = ClassIndex::load_or_build(&images, &labels, &flat).stats();
        assert_eq!(stats.images, 2);
        assert_eq!(stats.unlabeled, 1);
        assert_eq!(stats.images_per_class, vec![(0, 1), (2, 1)]);
    }
}
