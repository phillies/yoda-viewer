# YoDa Viewer – User Guide

YoDa Viewer is a browser (or desktop) tool to **review and edit Ultralytics
YOLO labels** – bounding boxes and segmentation polygons – for an image dataset.
It is built to stay responsive on large datasets (tens of thousands of 1080p
images) by indexing label files in a local database and loading thumbnails on
demand.

## Contents
1. [Quick start](#1-quick-start)
2. [Dataset layout](#2-dataset-layout)
3. [Configuration](#3-configuration)
4. [Running the app](#4-running-the-app)
5. [The interface](#5-the-interface)
6. [Finding images](#6-finding-images)
7. [Viewing labels](#7-viewing-labels)
8. [Editing labels](#8-editing-labels)
9. [Keyboard shortcuts](#9-keyboard-shortcuts)
10. [The index database and caches](#10-the-index-database-and-caches)
11. [Working with large datasets](#11-working-with-large-datasets)
12. [HTTP API reference](#12-http-api-reference)
13. [Docker](#13-docker)
14. [Troubleshooting & FAQ](#14-troubleshooting--faq)

---

## 1. Quick start
```bash
export YODA_IMAGE_BASE_PATH=/data/my-dataset/images
export YODA_LABEL_BASE_PATH=/data/my-dataset/labels
export YODA_CLASS_INFO_YAML=/data/my-dataset/data.yaml
cargo run -p yoda-web --features server
```
Open the URL printed in the log (default <http://127.0.0.1:8080>). Click
an image in the left tree to see its labels.

Requirements: Rust 1.85+. For `dx serve` / building the web bundle, install the
Dioxus CLI: `cargo install dioxus-cli`.

## 2. Dataset layout
Labels must **mirror the image folder structure**; only the root and the file
extension differ:

```
images/                      labels/
  train/                       train/
    car001.jpg        ->         car001.txt
    sub/car002.png    ->         sub/car002.txt   (for train/sub/car002.png)
  val/                         val/
    car900.webp       ->         car900.txt
```

- Supported image types: `jpg`, `jpeg`, `png`, `bmp`, `webp`.
- Hidden files and folders (names starting with `.`) are ignored.
- A missing label file simply means "no objects"; it is created on first save.
- Label lines follow Ultralytics YOLO format:
  - bounding box: `class cx cy w h`
  - polygon: `class x1 y1 x2 y2 x3 y3 ...`
  - all coordinates are normalised to 0–1.

### Class names (`YODA_CLASS_INFO_YAML`)
Ultralytics dataset YAML; only the `names:` key is used:
```yaml
names:
  0: car
  1: wheel
  2: door
```
A plain list (`names: [car, wheel, door]`) also follows the Ultralytics
convention of index = class id.

### Colours (`YODA_COLOR_MAP_YAML`, optional)
Maps class id to a colour. See `example/color_map.yaml` for the exact
format shipped with the repo. Classes without an entry get a deterministic
default colour.

## 3. Configuration
Settings come from environment variables or a `.env` file.

| Variable | Description | Default |
|---|---|---|
| `YODA_IMAGE_BASE_PATH` | Root folder of the images | required |
| `YODA_LABEL_BASE_PATH` | Root folder of the `.txt` labels | required |
| `YODA_CLASS_INFO_YAML` (or `YODA_CLASS_INFO`) | Dataset YAML with `names:` | – |
| `YODA_COLOR_MAP_YAML` (or `YODA_COLOR_MAP`) | Colour map YAML | – |
| `YODA_HOST` | Web server host | `127.0.0.1` |
| `YODA_PORT` | Web server port; if busy, the next free port is used | `8080` |

PowerShell: `$env:YODA_IMAGE_BASE_PATH = "D:\data\images"`.

> **Security note:** the server has no authentication. Keep the default
> `127.0.0.1` host unless you are on a trusted network or behind a reverse proxy.

## 4. Running the app
| Mode | Command |
|---|---|
| Web (server) | `cargo run -p yoda-web --features server` |
| Web (dev, hot reload) | `dx serve --package yoda-web --platform web` |
| Desktop | `cargo run -p yoda-desktop` (needs GTK/WebKit system libraries on Linux) |

On start the server scans the image tree and builds/updates the index
(see [section 10](#10-the-index-database-and-caches)). The first start on a big
dataset reads every label file and can take a while; later starts are fast.

## 5. The interface
```
┌────────────┬───────────────────────────────────┬──────────────┐
│ Left panel │ Toolbar                           │ Right panel  │
│  Search    │ ◀Prev Next▶ n/N Gallery | toggles │  Classes     │
│  Refresh   │ lock/draw controls                │  (legend)    │
│  Stats     ├───────────────────────────────────┤  Objects     │
│  Filter    │ Viewer  (or Gallery grid)         │  (list)      │
│  Tree      │                                   │              │
└────────────┴───────────────────────────────────┴──────────────┘
```

**Left panel** – search box, **⟳ Refresh**, **Stats**, class filter, file tree.
**Toolbar** – navigation, overlay toggles, edit lock and drawing tools.
**Viewer** – image with overlays; pan and zoom.
**Right panel** – class legend (visibility per class) and object list.

## 6. Finding images

### 6.1 File tree
Folders expand and collapse with a click; clicking an image opens it.

### 6.2 Filename search
Type into the search box. Matching is a case-insensitive substring match on the
dataset-relative path, so `train/car` matches `train/car25.jpg` and `sub` matches
every file under a folder named `sub`.
- Results are a flat list, capped at **500**; refine the query to narrow it.
- Each result shows a lazily loaded **thumbnail**.
- Clear the box to return to the tree.

### 6.3 Class filter
Click class chips to keep only images containing those classes.
- **Any** – at least one selected class is present.
- **All** – every selected class is present.
- **× Clear** resets the filter. A badge in the toolbar shows an active filter.
- Folders with no matching images are hidden; expand/collapse still works.
- Chips come from the class YAML. Classes appearing only in label files (not in
  the YAML) have no chip.
- The filter uses the index; use **⟳ Refresh** after external edits.

Search and class filter **combine** (both must match).

### 6.4 Prev / Next
**◀ Prev** and **Next ▶** step through the images matching the current search and
class filter, in tree order. The counter shows `position / total`. With no image
selected, Next opens the first match and Prev the last.

### 6.5 Gallery
**Gallery** replaces the viewer with a thumbnail grid of the same filtered list.
- 60 thumbnails per page with a pager (◀ ▶), so 50 000 images stay responsive.
- The grid opens on the page holding the current image; the current image is
  highlighted.
- Click a tile to open that image and return to the viewer. Hover for the full path.
- Thumbnails are generated on first view and cached (see section 10).

### 6.6 Stats
**▸ Stats** shows the number of images, the number of **unlabeled** images
(missing or empty label file) and the number of images containing each class.
Use it to spot class imbalance or unlabeled data, then filter by class to inspect.

## 7. Viewing labels
- **Mask / BBox** – show or hide polygon masks and bounding boxes.
- **Class ID / Class Name** – show text next to each object.
- **Zoom** – mouse wheel; **pan** – click and drag. The toolbar offers fit to
  screen, zoom in/out and 100 %.
- **Right panel → Classes**: every class present in the image with its colour.
  Untick to hide all objects of that class.
- **Right panel → Objects**: each object has
  - an **eye** to hide/show just that object,
  - a **class dropdown** to reassign it,
  - a badge: `[poly]` or `[bbox]`,
  - **Delete** (when editing is unlocked).

Visibility toggles are not stored on disk and reset when you switch images.

## 8. Editing labels
Editing is **locked by default** so you cannot change data by accident.

1. Click **Unlock Editing** (the status pill changes to *Unlocked*).
2. Change a class using the dropdown next to an object – the label file is
   saved immediately.
3. Select an object (click it on the canvas or in the list) and **Delete** it.
4. **Draw Polygon**: click vertices on the image; **Finish (Enter)** closes the
   polygon (clicking near the first vertex also closes it); **Cancel (Esc)**
   discards it. The new polygon gets saved to the label file.
5. Click **Lock Editing** to return to read-only mode.

Notes
- Every change writes the `.txt` file directly; there is no undo. Keep
  your dataset under version control or back it up.
- Saving updates the index immediately, so filters and stats stay correct.
- Bounding-box drawing and vertex editing are not available yet; draw a polygon
  or edit the file externally.

## 9. Keyboard shortcuts
Shortcuts are ignored while typing in an input, select or text area.

| Key | Action |
|---|---|
| `Delete` / `Backspace` | Delete the selected object (unlocked) |
| `Enter` | Finish the polygon being drawn |
| `Esc` | Cancel the polygon being drawn |

Prev/Next and Gallery are toolbar buttons only.

## 10. The index database and caches
All caches live **next to your labels** (inside `YODA_LABEL_BASE_PATH`); the
image folder is never written to. All are safe to delete – they are rebuilt.

| Path | Content |
|---|---|
| `<labels>/.yoda_index.sqlite` (+ `-wal`, `-shm`) | SQLite index: image path → label file mtime/size → class ids |
| `<labels>/.yoda_thumbs/` | Cached JPEG thumbnails |

### How the index stays fresh
- **Start-up**: scans images; re-reads only labels whose modification time or
  size changed (in parallel, one thread per CPU core); drops removed images.
- **Saving in YoDa**: the affected image is updated immediately.
- **External changes** (other tools, new/removed images): click **⟳ Refresh**
  or `POST /api/index/refresh`. The response reports `total`, `rescanned`
  and `removed`. Current search/filter are kept.
- **Read-only label folder**: the index is held in memory and rebuilt each start.
- The old `.yoda_class_index.json` is no longer used and can be deleted.

### Thumbnails
JPEG, longest edge 32–512 px (UI uses 128 and 256), keyed by path, modification
time and size – replaced images get a new thumbnail automatically. The browser
revalidates with ETags, so repeat visits cost a tiny 304 response.

Add both entries to `.gitignore` if your labels live in a git repository:
```
.yoda_index.sqlite*
.yoda_thumbs/
```

## 11. Working with large datasets
Rough guidance for ~50 000 1080p images:
- **First start** reads 50 000 label files; use an SSD/local disk for the labels.
  Later starts only `stat` each file.
- Put the dataset on **local storage**. Network shares make scanning and
  thumbnail generation noticeably slower.
- Use the **search box and class filter** rather than scrolling the tree;
  use **Gallery** for visual triage.
- The first Gallery page generates up to 60 thumbnails (decoding full-size
  JPEGs); subsequent visits are served from cache.
- Use **Stats** to find unlabeled images, and filter by a rare class to review
  only the relevant subset.
- Run Refresh after bulk external edits rather than restarting.

## 12. HTTP API reference
All endpoints are under `/api` and return JSON unless noted. Errors are
`{"code": "...", "message": "..."}` with an appropriate HTTP status.

| Method & path | Description |
|---|---|
| `GET /health` | Liveness: `{status:"ok"}` |
| `GET /tree`, `/tree/children`, `/tree/status` | Legacy tree endpoints |
| `GET /tree/flat` | Flat list of folder/image nodes |
| `GET /class-map` | Class id → name |
| `GET /color-map` | Class id → `[r,g,b]` |
| `GET /class-index` | Image path → class ids, plus all class ids |
| `GET /stats` | `{images, unlabeled, images_per_class:[[id,count],…]}` |
| `POST /index/refresh` | Rescan; returns `{total, rescanned, removed}` |
| `GET /image?image_path=` | Image bytes; `ETag`, honours `If-None-Match` (304) |
| `GET /image/thumbnail?image_path=&size=` | JPEG thumbnail (size 32–512, default 160) |
| `GET /image/metadata?image_path=` | Image dimensions |
| `GET /labels?image_path=` | Labels + dimensions |
| `PUT /labels?image_path=` | Replace labels: `{"labels":[…]}` |

`image_path` is relative to the image root; paths that escape the root are
rejected.

## 13. Docker
The repository `Dockerfile` builds the web bundle and server. Mount your dataset
and set the environment variables, and expose the port, e.g.:
```bash
docker build -t yoda-viewer .
docker run -p 8080:8080 -e YODA_HOST=0.0.0.0 \
  -e YODA_IMAGE_BASE_PATH=/data/images -e YODA_LABEL_BASE_PATH=/data/labels \
  -e YODA_CLASS_INFO_YAML=/data/data.yaml -v /my/dataset:/data yoda-viewer
```
Mount the label folder writable so edits and the index can be saved.
Binding to `0.0.0.0` exposes an unauthenticated editor – restrict access.

## 14. Troubleshooting & FAQ
| Symptom | Cause / fix |
|---|---|
| Blank page, API works | Web bundle missing – build with `dx build` / use `dx serve`; the server otherwise serves a read-only fallback page |
| Startup error about paths | `YODA_IMAGE_BASE_PATH` / `YODA_LABEL_BASE_PATH` unset or not a directory |
| Slow first start | The initial index build; later starts are incremental |
| Filter/Stats outdated | Labels changed outside YoDa – click **⟳ Refresh** |
| Image has no labels but file exists | Label path must mirror the image path with `.txt` extension |
| Class chip missing | Class not in the YAML `names:` |
| Class shown as `class N` | Id missing from the YAML |
| Index not persisted | Label folder not writable |
| Thumbnails slow first time | Generated on demand, then cached |
| Search shows only 500 hits | Intended cap; refine the query |
| Desktop build fails on Linux (`glib-sys`) | Install GTK/WebKit dev packages |
| Edited a label by mistake | No undo – restore from backup/version control |

**Does YoDa modify images?** No; only `.txt` labels (and its cache files in the
label folder).

**Can I use it on a polygon-only or bbox-only dataset?** Yes; both formats are
parsed and shown, per line.

**Is it multi-user?** No. Concurrent edits to the same label file by several
sessions can overwrite each other.
