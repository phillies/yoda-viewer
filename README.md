# YoDa Viewer
YOLO Dataset viewer — review and edit Ultralytics YOLO segmentation & bounding-box labels in your browser.

## Installation
```bash
uv pip install .
```

## Run
```bash
yoda
```
The server starts on port **8080** by default. If that port is in use it automatically increments until a free port is found.

### Configuration
All settings are read from environment variables (or a `.env` file):

| Variable | Description | Default |
|---|---|---|
| `YODA_IMAGE_BASE_PATH` | Root folder that contains the images | (required) |
| `YODA_LABEL_BASE_PATH` | Root folder that contains the YOLO `.txt` label files (must mirror the image folder structure) | (required) |
| `YODA_CLASS_INFO_YAML` | Path to dataset YAML with class names (Ultralytics format, `names:` key) | — |
| `YODA_COLOR_MAP_YAML` | Path to a color-map YAML (`class_id: "#RRGGBB"`) | — |
| `YODA_HOST` | Host for the uvicorn server | `0.0.0.0` |
| `YODA_PORT` | Port for the uvicorn server | `8080` |

## Features

### Viewing
- **File tree** on the left shows the image folder hierarchy; click any image to open it.
- **Segmentation masks** and/or **bounding boxes** are drawn as SVG overlays on top of the image.
- Toggle overlays with the **Show Bounding Boxes** / **Show Segmentation** checkboxes.
- Toggle **Class ID** and **Class Name** display on each object.
- **Zoom & pan**: mouse-wheel zoom, click-and-drag pan, plus toolbar buttons for *Fit to screen*, *Zoom in*, *Zoom out*, and *100 %*.

### Right drawer — Class legend
- Every class present in the current image is listed with its colour and name.
- Each class has a **checkbox** — uncheck it to hide *all* objects of that class at once.

### Right drawer — Object list
- Each detected object is shown with:
  - An **eye icon** (👁) to toggle visibility of that single object.
  - A **class dropdown** to reassign the object to a different class. Changes are **saved to disk** immediately.
  - A **type badge** (`[poly]` for segmentation, `[bbox]` for bounding box).

### Editing
- **Change class**: select a new class from the dropdown next to any object — the label file is updated on disk automatically.
- **Hide / show by class**: use the class-legend checkboxes.
- **Hide / show individual objects**: use the eye-icon buttons in the object list.
