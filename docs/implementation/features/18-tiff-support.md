# TIFF Image Support

> Source: next-features.md §3.9 · Effort: S (more than the one-liner it looks like)

## Problem

Datasets from scientific/GIS/industrial sources ship `.tif`/`.tiff`. The `image` crate
supports TIFF; the app's extension filters don't.

## All the places extensions live (the actual work)

The extension list is duplicated in **three** places today — consolidate while touching this:

1. `IMAGE_EXTENSIONS` in `crates/yoda-data/src/lib.rs:19` (tree scan + flat index).
2. `is_image_path` in `crates/yoda-web/src/lib.rs:872-879` (API request validation) — an
   independent hardcoded `matches!` list.
3. `mime_type_for_image` in `crates/yoda-web/src/lib.rs:973-987`.

Consolidation: move to `yoda-core` (or `yoda-data`, but core has no fs deps and both web+data
depend on it):

```rust
pub const IMAGE_EXTENSIONS: &[&str] = &["jpg","jpeg","png","bmp","webp","tif","tiff"];
pub fn is_supported_image_ext(ext: &str) -> bool     // lowercase compare
pub fn mime_type_for_ext(ext: &str) -> &'static str  // tiff → "image/tiff"
```

Rewire the three sites (note yoda-data's list stores dot-prefixed strings — normalize on the
helper's contract: extension without dot, lowercased).

## The catch: browsers don't render TIFF

`<img src="/api/image?...">` with `image/tiff` shows a broken image in every mainstream
browser. So serving raw bytes is not enough — TIFF requires **server-side transcoding**:

- In `image_bytes` (`yoda-web`): when the extension is tif/tiff, decode with the `image`
  crate and re-encode to PNG (lossless — labels are drawn over it; JPEG would smear):

```rust
let img = image::load_from_memory(&bytes)?;          // or image::open before reading raw
let mut out = Vec::new();
img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png)?;
// serve with image/png
```

- Enable the `tiff` feature on the workspace `image` dependency (currently
  `default-features = false, features = ["bmp","jpeg","png","webp"]` — `png` encode already
  enabled ✓).
- Transcoding cost: large TIFFs are slow to decode — run inside `spawn_blocking`
  (performance/05) and let performance/06's ETag (validator = source file mtime+size) make it
  once-per-client. Optional later: disk-cache transcodes next to thumbnails (features/22).
- 16-bit / multi-channel TIFFs: `DynamicImage` conversion to 8-bit RGB via `img.into_rgb8()`
  loses depth — acceptable for viewing; document. Truly exotic TIFFs (tiled BigTIFF, float32)
  may fail to decode → existing error path returns 500 with message; fine.

`image_dimensions` works for TIFF without decode (header read) once the feature is on ✓.

## Testing

- `yoda-data`: extension test extends the `all_image_extensions_included` case.
- `yoda-web` Axum test: request a generated `.tif` fixture → 200, `content-type: image/png`,
  body decodes as PNG with matching dimensions; `/api/image/metadata` returns tif dims.
- Manual: browser shows a TIFF image with overlays aligned.

## Risks

- `image`'s tiff decoder pulls extra deps (weight: small). No API risk.
