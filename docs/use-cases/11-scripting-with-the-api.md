# Use Case 11 — Script Against the HTTP API

> **Goal:** use YoDa's backend headlessly — dataset sanity checks in CI, extracting label
> data into notebooks, or bulk edits from scripts, all against the same code paths the UI
> uses.
> **Actor:** ML engineer automating dataset chores.
> **Mode:** API only (the UI is optional; the SSR-fallback/no-bundle server serves the full
> API).

## Setup

Run the server against the dataset (any deployment from use case
[09](09-serve-a-remote-dataset.md); no `dx build` needed for API-only use):

```bash
YODA_IMAGE_BASE_PATH=… YODA_LABEL_BASE_PATH=… cargo run -p yoda-web --features server
```

All endpoints are under `http://host:port/api` — full reference in the
[README](../../README.md#http-api-reference). Errors come back as JSON
`{"code": …, "message": …}` with proper status codes; paths outside the image root are
rejected with `403`.

## Recipes

**Inventory: every image and its dimensions**

```bash
curl -s localhost:8080/api/tree/flat |
  jq -r '.nodes[] | select(.kind=="Image") | .path'
```

**Which images contain class 3? (uses the same index as the UI filter)**

```bash
curl -s localhost:8080/api/class-index |
  jq -r '.entries | to_entries[] | select(.value | index(3)) | .key'
```

**Find unlabeled images**

```bash
curl -s localhost:8080/api/class-index |
  jq -r '.entries | to_entries[] | select(.value == []) | .key'
```

**Pull one image's labels as JSON (parsed, with pixel coordinates precomputed)**

```bash
curl -s 'localhost:8080/api/labels?image_path=train/img001.jpg' | jq '.labels'
```

**Bulk edit — reassign class 7→2 on one image (Python)**

```python
import requests
base = "http://localhost:8080/api"
p = "train/img001.jpg"
data = requests.get(f"{base}/labels", params={"image_path": p}).json()
for l in data["labels"]:
    if l["class_id"] == 7:
        l["class_id"] = 2
requests.put(f"{base}/labels", params={"image_path": p},
             json={"labels": data["labels"]}).raise_for_status()
```

The PUT rewrites the label file, updates the class index, and returns the freshly re-parsed
labels — compare them to what you sent as a built-in verification step.

**CI smoke check** — `GET /api/health` returns `{"status":"ok","version":…}`; combine with
`tree/status` (`{node_count, image_count}`) to assert a dataset mounted correctly.

## Rules of engagement

- **GET-then-PUT the full label set.** The PUT replaces the whole file; always start from a
  fresh GET, mutate, and send everything back. Concurrent writers are last-writer-wins.
- Coordinates: `normalized_coords` is the source of truth; `pixel_points`/`pixel_bbox` are
  derived conveniences. If you construct labels yourself, populate all fields consistently
  (safest: GET an existing object of the same type and mirror its shape).
- Cross-origin browser calls (e.g. from a hosted notebook) are blocked — no CORS headers yet
  ([designed](../implementation/features/17-cors-support.md)); call from server-side code or
  the same origin.
- No auth ([designed](../implementation/features/12-read-only-mode.md)); treat the endpoint
  like a writable network share.

## Related use cases

- [03 — Find images by class](03-find-images-by-class.md) (the UI equivalent of the index
  queries)
- [09 — Serve a remote dataset](09-serve-a-remote-dataset.md)
