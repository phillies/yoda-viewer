# Use Case 09 — Serve a Dataset from a Remote Machine or Docker

> **Goal:** run YoDa where the data lives (training server, NAS-adjacent box) and review it
> from your local browser — no dataset copying.
> **Actor:** ML engineer with datasets on shared infrastructure.
> **Mode:** deployment.

## Option A — Docker (recommended for servers)

```bash
docker build -t yoda-viewer .          # builds WASM bundle + server into one image
docker run -d --name yoda \
  -p 8080:8080 \
  -v /datasets/carparts/images:/data/images:ro \
  -v /datasets/carparts/labels:/data/labels \
  -v /datasets/carparts/carparts-seg.yaml:/data/classes.yaml:ro \
  -e YODA_CLASS_INFO_YAML=/data/classes.yaml \
  yoda-viewer
```

- The image defaults to `YODA_HOST=0.0.0.0`, `YODA_PORT=8080`,
  `/data/images` + `/data/labels`.
- Mount labels **read-write**: YoDa writes both label edits and its class-index cache
  (`.yoda_class_index.json`) into the label root. A read-only label mount makes editing fail
  and the cache re-scan on every start.
- Browse to `http://<server>:8080`.

## Option B — bare binary on the remote host

```bash
# on the server (once): build with the web bundle
cargo install dioxus-cli && dx build --release -p yoda-web
cargo build --release -p yoda-web --features server
# run
YODA_HOST=0.0.0.0 YODA_PORT=8080 \
YODA_IMAGE_BASE_PATH=/datasets/carparts/images \
YODA_LABEL_BASE_PATH=/datasets/carparts/labels \
./target/release/yoda-web
```

The built `public/` assets must sit next to the binary (that's where the server looks) —
otherwise you get the read-only SSR fallback page instead of the app.

## Option C — SSH tunnel (zero exposure)

Keep the server bound to `127.0.0.1` (the default) and tunnel:

```bash
ssh -L 8080:127.0.0.1:8080 user@server
# then open http://localhost:8080 locally
```

This is the **recommended pattern today** given the security posture below.

## Security posture — read before exposing a port

The current version has **no authentication, no authorization, no read-only switch, and no
TLS**. `PUT /api/labels` is open: anyone who can reach the port can rewrite your labels.
Path traversal is guarded (requests outside the image root get `403`), but that's the only
guard. Therefore:

- Prefer the SSH tunnel (Option C) or a VPN/trusted network.
- If you must expose it, front it with a reverse proxy providing auth + TLS.
- A `YODA_READ_ONLY` flag and a shared-token option are designed but not built
  ([read-only mode](../implementation/features/12-read-only-mode.md),
  [CORS](../implementation/features/17-cors-support.md)).

## Operational notes

- **First start on a big dataset** scans every label file to build the class index — the
  server accepts connections only after the scan (tens of seconds; no progress indication —
  [designed](../implementation/features/07-index-build-progress.md)). Subsequent starts load
  the cache.
- **Concurrent viewers work; concurrent editors are last-writer-wins** with no conflict
  detection — coordinate socially ([designed](../implementation/bold/04-multi-user-review.md)).
- Labels edited by other processes while YoDa runs aren't picked up until restart
  ([designed](../implementation/features/10-class-index-refresh.md)).
- Images re-download on every view (no HTTP caching yet) — noticeable on slow links
  ([designed](../implementation/performance/06-http-caching-compression.md)).

## Related use cases

- [10 — Desktop local review](10-desktop-local-review.md)
- [11 — Scripting with the API](11-scripting-with-the-api.md)
