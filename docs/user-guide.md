# YoDa Viewer – User Guide

## 1. Overview
YoDa Viewer shows and edits Ultralytics YOLO labels (bounding boxes and
polygons) for an image dataset in your browser. It is designed to stay
responsive on large datasets (tens of thousands of 1080p images).

## 2. Setup
1. Install Rust 1.85+.
2. Set environment variables (or a `.env` file):

| Variable | Meaning |
|---|---|
| `YODA_IMAGE_BASE_PATH` | Root folder of the images (required) |
| `YODA_LABEL_BASE_PATH` | Root folder of the `.txt` labels; mirrors the image folder structure (required) |
| `YODA_CLASS_INFO_YAML` | Dataset YAML with `names:` |
| `YODA_COLOR_MAP_YAML` | Optional `class_id: "#RRGGBB"` colors |
| `YODA_HOST` / `YODA_PORT` | Server address (default `127.0.0.1:8080`) |

3. Run `cargo run -p yoda-web --features server` and open the printed URL.

## 3. Navigating a large dataset
### File tree
Folders expand/collapse; click an image to open it.

### Filename search
Type in the search box above the tree. Matching is a case-insensitive
substring match on the dataset-relative path (`train/car` finds
`train/car25.jpg`). Results are a flat list, capped at 500; refine the query
if you hit the cap. Search combines with the class filter.

When a search is active, each result shows a thumbnail (lazy-loaded).
Thumbnails are JPEGs cached in `<YODA_LABEL_BASE_PATH>/.yoda_thumbs/`, keyed by
path, modification time and size, so they regenerate automatically when an
image changes. The folder is safe to delete.

### Prev / Next and Gallery
The toolbar has **◀ Prev** / **Next ▶** buttons that step through the images
matching the current search and class filter (tree order), with a position
counter. **Gallery** replaces the viewer with a thumbnail grid of the same
list, 60 images per page (so the page stays fast with 50k images). Click a
tile to open the image; the grid opens on the page of the current image.

### Stats panel
Click **▸ Stats** above the tree to see the image count, the number of
unlabeled images (empty or missing label file) and, per class, how many images
contain it. Stats come from the index; click Refresh after external edits.

### Class filter
Select class chips to show only images containing those classes.
- **Any**: at least one selected class is present.
- **All**: every selected class is present.
Use **× Clear** to reset. Note: only classes listed in the class YAML are shown as chips.

## 4. The index database
On startup the server builds an index of which classes occur in each image and
stores it in `<YODA_LABEL_BASE_PATH>/.yoda_index.sqlite` (hidden; safe to
delete, it is rebuilt).

- The first build reads every label file (parallelised across CPU cores).
- Later starts re-read only label files whose modification time or size changed,
  and drop entries for removed images.
- Saving labels in the viewer updates the index immediately.
- If you edit labels or add/remove images **outside** the viewer while it is
  running, click **⟳ Refresh** (or `POST /api/index/refresh`) to resync the
  tree and index. Refresh keeps the current filter and search.
- The label folder must be writable for the cache to persist; otherwise the
  index is kept in memory only and rebuilt each start.
- The old `.yoda_class_index.json` is no longer used and can be deleted.

## 5. Viewing and editing
- Toggle bounding boxes, segmentation, class IDs and class names.
- Zoom with the mouse wheel, pan by dragging; use the toolbar for fit/100 %.
- Right drawer: per-class visibility checkboxes; per-object eye toggle,
  class dropdown (saved to disk immediately) and type badge (`[poly]`/`[bbox]`).

## 6. HTTP API (summary)
| Endpoint | Purpose |
|---|---|
| `GET /api/health` | Liveness |
| `GET /api/tree/flat` | Flat dataset tree |
| `GET /api/class-index` | Image → class IDs map |
| `POST /api/index/refresh` | Rescan; returns `{total, rescanned, removed}` |
| `GET/PUT /api/labels?image_path=` | Load / save labels |
| `GET /api/image?image_path=` | Image bytes (ETag; supports `If-None-Match` → 304) |
| `GET /api/image/thumbnail?image_path=&size=` | Cached JPEG thumbnail (size 32–512, default 160) |
| `GET /api/stats` | `{images, unlabeled, images_per_class}` |

## 7. Troubleshooting
- **Slow first start**: expected while the index is built once.
- **Filter results out of date**: click Refresh.
- **Thumbnails slow the first time**: they are generated on demand, then cached.
- **Index not persisted**: check write permission on the label folder.
- **Classes missing as chips**: add them to the class YAML.
