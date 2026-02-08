# YoDa
YOLO Dataset viewer

## Installation
Simply install the package with `[uv] pip install .`.

## Run
The dev server (with reload) is started with `yoda-dev`, the prod server without hot reloading with `yoda`.
It starts the server on port 8080 by default.

### Configuration
Environment variables to configure yoda:
```bash
YODA_IMAGE_BASE_PATH=/path/to/images
YODA_LABEL_BASE_PATH=/path/to/images
YODA_CLASS_INFO_YAML=/optional/path/to/dataset/yaml/with/all/class/names
YODA_COLOR_MAP_YAML=/optional/path/to/colormap
YODA_HOST=/host/parameter/for/uvicorn
YODA_PORT=/port/parameter/for/uvicorn
```
