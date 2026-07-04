# Class Filter – Design Document

## 1. Goal

Add a **Class Filter** that lets the user select one or more classes and shows
only images that contain at least one annotation belonging to those classes.
The file tree (left panel) collapses to the matching subset and a clear "filter
active" indicator is shown.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│  Browser (yoda-ui)                                      │
│                                                         │
│  ClassFilterBar  ──► AppAction::SetClassFilter(...)     │
│  FlatNodeView    ──► filtered_visible_rows (memo)       │
└───────────────────────────────────────────┬─────────────┘
                                            │ REST
┌───────────────────────────────────────────▼─────────────┐
│  yoda-web  (server feature)                             │
│                                                         │
│  GET /api/class-index  ──► ClassIndexResponse           │
│  (backed by ClassIndexCache in BackendState)            │
└───────────────────────────────────────────┬─────────────┘
                                            │
┌───────────────────────────────────────────▼─────────────┐
│  yoda-data                                              │
│                                                         │
│  ClassIndexCache  (builds + persists label-class sets)  │
└─────────────────────────────────────────────────────────┘
```

---

## 3. Persistent Class Index Cache  (`yoda-data`)

### 3.1 Data model

```rust
// yoda-data/src/class_index.rs

/// Sparse map: dataset-relative image path → sorted set of class IDs
/// that appear in the corresponding label file.
///
/// Serialised to / deserialised from a JSON file on disk so the index
/// survives restarts without re-scanning every label file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassIndex {
    /// Key: dataset-relative path (forward slashes, e.g. "train/car25.jpg")
    /// Value: sorted, deduplicated list of class IDs present in that image
    pub entries: HashMap<String, Vec<u32>>,
}
```

### 3.2 Cache file location

```
<label_base_path>/.yoda_class_index.json
```

A dot-prefixed file so it is hidden on Unix and ignored by `scan_dataset_tree`
(which already skips entries starting with `.`).

### 3.3 Build / refresh logic

```rust
impl ClassIndex {
    /// Load the persisted index, scan any missing or stale entries, and save.
    pub fn load_or_build(
        image_root: &Path,
        label_root: &Path,
        flat_index: &FlatIndex,
    ) -> Result<Self, RepositoryError> {
        let cache_path = label_root.join(".yoda_class_index.json");
        let mut index = Self::load_from_disk(&cache_path).unwrap_or_default();

        let mut dirty = false;
        for node in flat_index.nodes.iter().filter(|n| n.kind == NodeKind::Image) {
            if index.entries.contains_key(&node.path) {
                continue; // already cached
            }
            // Derive the label path from the image path
            let image_abs = image_root.join(&node.path);
            let label_abs = map_image_to_label_path(&image_abs, image_root, label_root)?;
            let class_ids = extract_class_ids_from_label_file(&label_abs);
            index.entries.insert(node.path.clone(), class_ids);
            dirty = true;
        }

        // Prune entries for images that no longer exist in the flat index
        let known_paths: HashSet<&str> = flat_index.nodes.iter()
            .filter(|n| n.kind == NodeKind::Image)
            .map(|n| n.path.as_str())
            .collect();
        let before = index.entries.len();
        index.entries.retain(|k, _| known_paths.contains(k.as_str()));
        dirty |= index.entries.len() != before;

        if dirty {
            index.save_to_disk(&cache_path)?; // best-effort; log on failure
        }
        Ok(index)
    }

    /// Returns all class IDs present anywhere in the dataset (union of all
    /// entries), useful for populating the filter UI.
    pub fn all_class_ids(&self) -> BTreeSet<u32> {
        self.entries.values().flatten().copied().collect()
    }

    /// Returns dataset-relative paths of images that contain **all** of the
    /// given required classes (AND semantics) or **any** (OR semantics).
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
            .filter(|(_, classes)| {
                let set: BTreeSet<u32> = classes.iter().copied().collect();
                match mode {
                    FilterMode::Any => required.iter().any(|id| set.contains(id)),
                    FilterMode::All => required.iter().all(|id| set.contains(id)),
                }
            })
            .map(|(path, _)| path.as_str())
            .collect()
    }
}

/// Controls how multiple selected classes are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FilterMode {
    /// Show images that contain at least one of the selected classes.
    #[default]
    Any,
    /// Show images that contain every one of the selected classes.
    All,
}

fn extract_class_ids_from_label_file(label_path: &Path) -> Vec<u32> {
    let Ok(content) = fs::read_to_string(label_path) else { return Vec::new() };
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
```

### 3.4 Incremental invalidation

When `save_labels` is called for an image, the server should update the cache
entry for that image in-place (see §5.2). No full rescan is needed.

---

## 4. Backend API  (`yoda-web`)

### 4.1 New `BackendState` fields

```rust
pub struct BackendState {
    repository: LocalDatasetRepository,
    image_root: PathBuf,
    flat_index: Arc<FlatIndex>,
    // NEW ─────────────────────────────────────────────────────────────
    class_index: Arc<RwLock<ClassIndex>>,
}
```

`ClassIndex` is wrapped in `Arc<RwLock<...>>` so it can be updated in-place
when labels are saved without rebuilding `BackendState`.

`BackendState::from_settings` calls `ClassIndex::load_or_build` during startup.
Because this scans label files it can take a few hundred milliseconds for large
datasets; it runs synchronously but only once.

### 4.2 New endpoint: `GET /api/class-index`

```
GET /api/class-index
    ?filter_classes=0,1,3   (optional, comma-separated class IDs)
    &filter_mode=any         (optional: "any" | "all", default "any")
```

**Response** (always returned even with no query params):

```jsonc
{
  // For every dataset-relative image path, the sorted list of class IDs:
  "entries": {
    "train/car25.jpg": [0, 2],
    "val/car4.jpg":    [0, 1, 3]
  },
  // Convenience: the union of all class IDs in the dataset
  "all_class_ids": [0, 1, 2, 3],
  // When filter_classes is provided: paths that match the filter
  "matching_paths": ["train/car25.jpg"]
}
```

The full `entries` map is sent once on startup and cached client-side. Matching
is then done entirely in the browser via the memo computed in §6.3.  Sending
the full index enables offline-capable filtering with zero round-trips while the
user changes the filter.

**Implementation note**: if the dataset is very large (>50 k images), the full
map transfer cost (~5–10 KB gzipped for typical datasets) is still negligible.
For extreme cases a pagination strategy can be added later.

### 4.3 Router addition

```rust
.route("/class-index", get(class_index_handler))
```

```rust
async fn class_index_handler(
    Extension(state): Extension<Arc<BackendState>>,
) -> Json<ClassIndexResponse> {
    let index = state.class_index.read().unwrap();
    let all_class_ids = index.all_class_ids().into_iter().collect();
    Json(ClassIndexResponse {
        entries: index.entries.clone(),
        all_class_ids,
    })
}
```

### 4.4 Updated `save_labels` handler

After writing the label file, update the class index entry and persist:

```rust
// After services.persist_labels(...)
{
    let image_rel = dataset_relative_path(&state.image_root, &image_path);
    let mut index = state.class_index.write().unwrap();
    let new_ids = extract_class_ids(&payload.labels);
    index.entries.insert(image_rel, new_ids);
    let cache_path = state.repository.label_root().join(".yoda_class_index.json");
    let _ = index.save_to_disk(&cache_path); // log on failure; don't fail request
}
```

---

## 5. Application State  (`yoda-app`)

### 5.1 New state fields

```rust
pub struct AppState {
    // ... existing fields unchanged ...

    // NEW ─────────────────────────────────────────────────────────────
    /// Classes to filter by. Empty = no filter active.
    pub filter_classes: BTreeSet<u32>,
    /// Whether ALL selected classes must be present or ANY.
    pub filter_mode: FilterMode,
    /// Pre-fetched from server; maps dataset-relative path → class IDs.
    pub class_index: HashMap<String, Vec<u32>>,
}
```

### 5.2 New actions

```rust
pub enum AppAction {
    // ... existing variants ...

    ClassIndexLoaded(HashMap<String, Vec<u32>>),
    SetFilterClass { class_id: u32, selected: bool },
    ClearClassFilter,
    SetFilterMode(FilterMode),
}
```

The reducer for these actions is trivial: update the corresponding state fields
and return `ActionResult::applied()`. No `AppEffect` is needed (the filter is
pure client-side).

---

## 6. UI  (`yoda-ui`)

### 6.1 `ClassFilterBar` component

A new component rendered **above** the file tree, between the "Dataset" title
and the tree scroll area.

```
┌─────────────────────────────┐
│  DATASET            [🔍 ×]  │
│  ┌──────────────────────┐   │
│  │ Filter: ○Any  ○All   │   │
│  │ [car] [wheel] [door] │   │
│  │ ○0 ○1 ○2 ○3 ○4      │   │
│  └──────────────────────┘   │
│  (tree nodes…)              │
└─────────────────────────────┘
```

- Renders one toggle button per class (using `class_map` + `color_map`).
- Selected classes are highlighted with their class color.
- An "Any / All" radio toggle at the top.
- A `×` clear-filter button shown only when a filter is active.
- When no classes are selected → tree shows all images (no filter).

### 6.2 Filtered visible rows memo

```rust
let filtered_visible_rows = use_memo(move || {
    let state = app_state();
    let rows = visible_rows(); // existing memo

    if state.filter_classes.is_empty() {
        return rows;
    }

    // Build the set of image paths that match the filter
    let matching: HashSet<String> = state
        .class_index
        .iter()
        .filter(|(_, class_ids)| {
            let id_set: BTreeSet<u32> = class_ids.iter().copied().collect();
            match state.filter_mode {
                FilterMode::Any => state.filter_classes.iter().any(|id| id_set.contains(id)),
                FilterMode::All => state.filter_classes.iter().all(|id| id_set.contains(id)),
            }
        })
        .map(|(path, _)| path.clone())
        .collect();

    // Keep a folder row if any of its descendants are in the matching set
    let matching_folder_ids = build_matching_folder_ids(&rows, &matching);

    rows.into_iter()
        .filter(|row| match row.kind {
            NodeKind::Image => matching.contains(&row.path),
            NodeKind::Folder => matching_folder_ids.contains(&row.id),
        })
        .collect()
});
```

`build_matching_folder_ids` walks the tree bottom-up and marks any folder whose
subtree contains at least one matching image.  It is a pure function of rows +
matching set, executed client-side in O(n).

### 6.3 Class fetch on startup

Add a single fetch alongside the existing `fetch_class_map` / `fetch_color_map`
calls in the startup `use_effect`:

```rust
match fetch_class_index(&api_base_value).await {
    Ok(entries) => {
        let effects = reduce_state(app_state, AppAction::ClassIndexLoaded(entries));
        run_effects(api_base_value.clone(), app_state, effects);
    }
    Err(error) => set_status_error(app_state, error),
}
```

### 6.4 Filter activity badge in status bar

When a filter is active, show a dismissable pill in the status bar:

```
Image: …  Dimensions: …  Objects: …  Mode: …  [Filter: car, wheel  ×]
```

---

## 7. Fallback Viewer  (`yoda-web`)

The server-rendered fallback viewer (`render_fallback_viewer`) does not need to
support interactive filtering. It can optionally accept a
`?filter_classes=0,1,2` query parameter and render only matching tree items,
re-using `ClassIndex::matching_images` server-side. This is a low-priority
addition and can be deferred.

---

## 8. Implementation Checklist

### Phase 1 – Backend cache & API  (`yoda-data` + `yoda-web`)

- [ ] Add `FilterMode` enum to `yoda-data`
- [ ] Add `ClassIndex` struct with `load_or_build`, `matching_images`,
      `save_to_disk`, `load_from_disk`, `all_class_ids`, `extract_class_ids_from_label_file`
- [ ] Add `class_index` field to `BackendState`; build in `from_settings`
- [ ] Add `GET /api/class-index` endpoint and `ClassIndexResponse` type
- [ ] Update `save_labels` to invalidate the cache entry for the modified image
- [ ] Add `serde_json` (already in workspace deps) import to `yoda-data`
- [ ] Unit-test `ClassIndex::load_or_build` with `tempfile`

### Phase 2 – App state & actions  (`yoda-app`)

- [ ] Add `filter_classes`, `filter_mode`, `class_index` to `AppState`
- [ ] Add `ClassIndexLoaded`, `SetFilterClass`, `ClearClassFilter`,
      `SetFilterMode` to `AppAction`
- [ ] Implement reducer branches for the new actions
- [ ] Update `AppState::Default` (filter_classes = empty, filter_mode = Any)

### Phase 3 – UI  (`yoda-ui`)

- [ ] Add `fetch_class_index` async function
- [ ] Dispatch `ClassIndexLoaded` on startup (alongside class-map / color-map)
- [ ] Add `ClassFilterBar` component
- [ ] Add `filtered_visible_rows` memo replacing `visible_rows` in the tree
- [ ] Implement `build_matching_folder_ids` helper
- [ ] Show filter badge in status bar
- [ ] CSS: new `.filter-bar`, `.filter-chip`, `.filter-chip.active` rules

### Phase 4 – Desk-check / tests

- [ ] Integration test: `/api/class-index` returns correct entries for sample dataset
- [ ] Unit test: `build_matching_folder_ids` with nested folder structure
- [ ] Manual E2E: filter on one class → only matching images shown; clear → all images return

---

## 9. Key Design Decisions

| Decision | Rationale |
|---|---|
| Cache file next to labels, not in a separate state dir | Label root is already the write-able metadata location; keeps everything together |
| Full index sent to client once | Enables zero-round-trip filtering as user changes selection; dataset index is small (~100 B/image) |
| Filter is pure client state, no server round-trip | Instant response; avoids chat between browser and server on every checkbox click |
| `BTreeSet<u32>` for class IDs in filter | Deterministic ordering, fast contains-check, serialises cleanly |
| Dot-prefix for cache file | Automatically excluded by `scan_dataset_tree` (already skips `.` entries) |
| `RwLock` around `ClassIndex` in server | Allows `save_labels` to update a single entry without rebuilding the whole index |
| `FilterMode::Any` as default | More intuitive for "show me all images with a car" use-case |
| Folder rows kept when subtree matches | Standard file-tree filter UX (VS Code, JetBrains etc.) |
