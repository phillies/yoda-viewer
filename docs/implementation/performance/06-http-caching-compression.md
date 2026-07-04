# HTTP Caching Headers + Response Compression

> Source: optimizations-and-features.md §3.6 · Effort: S
> Depends on: nothing · Biggest bang-for-buck perf item alongside performance/01

## Problem

- `GET /api/image` (`crates/yoda-web/src/lib.rs:429-447`) returns raw bytes with only a
  `Content-Type`. No `Cache-Control`, no `ETag`, no `Last-Modified` → every image revisit
  (prev/next review flows especially) re-downloads megabytes.
- No compression layer anywhere: `/api/tree/flat` and `/api/class-index` are large, highly
  repetitive JSON (multi-MB on big datasets) sent uncompressed. `tower-http` is already a
  dependency but only with `fs`/`trace` features (`Cargo.toml:50`).

## Design

### 1. Compression (one-liner + feature flag)

- `Cargo.toml`: `tower-http = { version = "0.6", features = ["fs", "trace",
  "compression-gzip", "compression-br"] }`
- In `build_router` / `build_api_router`:
  `.layer(CompressionLayer::new())` on the API router.
- **Exclude images**: JPEG/PNG/WebP don't compress and burn CPU. `CompressionLayer` respects
  `content-type` via a predicate:
  `CompressionLayer::new().compress_when(SizeAbove::new(1024).and(NotForContentType::IMAGES))`
  (tower-http ships `DefaultPredicate` that already skips images — verify and rely on it).

### 2. Conditional requests for images

Dataset images are immutable-in-practice but *can* be swapped on disk, so use validation
(ETag) rather than long `max-age`:

```rust
let meta = fs::metadata(&image_path)?;
let etag = format!("\"{}-{}\"", meta.len(),
    meta.modified()?.duration_since(UNIX_EPOCH)?.as_secs());
if request_if_none_match == Some(etag) { return 304 }
headers: ETag, Cache-Control: private, no-cache   // no-cache = revalidate each use, 304 is cheap
```

- Extract `If-None-Match` via `axum::http::HeaderMap` parameter in the handler.
- mtime+size is a sufficient validator here; no hashing needed.
- Result: first view downloads; every later view is a ~0-byte 304. Prev/next paging becomes
  instant even over Wi-Fi.

Optionally add the same to `/api/labels` (validator = label-file mtime+size, or absent file →
weak etag `W/"empty"`), which also gives external-edit freshness for free.

### 3. Static asset caching (WASM bundle)

`serve_dioxus_application` serves hashed assets from `public/`; dx output filenames are
content-hashed → verify response headers and, if missing, wrap with
`SetResponseHeaderLayer` adding `Cache-Control: public, max-age=31536000, immutable` for
`/assets/*` (confirm actual asset route prefix from a `dx build` output before hardcoding).

## Testing

- Axum tests: request image → capture `ETag`; repeat with `If-None-Match` → assert 304 and
  empty body. Modify file mtime → 200 again.
- Compression: request `/api/tree/flat` with `Accept-Encoding: gzip` → assert
  `content-encoding: gzip` and that the body inflates to the original JSON. Request an image
  with the same header → assert **no** content-encoding.
- Manual sanity via browser devtools on the example dataset.

## Risks

- `TraceLayer` + `CompressionLayer` ordering: put compression **outside** (added last) so
  traces log uncompressed sizes; either order works functionally.
- 304 handling must skip the `fs::read` entirely — do the metadata check before reading bytes
  (also a small perf win per request).
