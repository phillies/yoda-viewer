# XML-Escape Class Names in the SVG Overlay

> Source: optimizations-and-features.md §2.1 · Effort: XS
> Depends on: nothing · Related: performance/02 (inline SVG) removes the data-URI failure mode
> but escaping is still required there.

## Problem

`render_text_label` (`crates/yoda-core/src/render.rs:149-182`) interpolates the class name
(from the user's dataset YAML) directly into `<text …>{text}</text>`. A class named `R&D` or
`<5cm` yields invalid XML. Because the overlay is delivered as a single
`data:image/svg+xml` image (`render_overlay_data_uri`, `crates/yoda-ui/src/lib.rs:1561`;
`build_overlay_data_uri`, `crates/yoda-web/src/lib.rs:829`), the browser rejects the *entire*
overlay — every label disappears, with no error surfaced.

## Design

1. Move the existing `escape_html` from `crates/yoda-web/src/lib.rs:910-917` into `yoda-core`
   (e.g. `pub fn escape_xml(&str) -> String` in a small `text.rs` module — the five entities
   `& < > " '` are identical for XML). Re-export from `yoda_core::`.
2. In `render_text_label`, escape `text` before interpolation. Note `text_width` is computed
   from `text.chars().count()` — compute width from the **unescaped** string first, then
   escape for output, so `&amp;` doesn't inflate the background rect.
3. `yoda-web` replaces its private `escape_html`/`escape_attr` with the shared function
   (`escape_attr` can stay as an alias).

## Testing

- Unit test in `render.rs`: class map `{0: "R&D <x>"}` → output contains
  `R&amp;D &lt;x&gt;`, contains no raw `&` followed by space, and the rect width matches the
  9-char unescaped length.
- Update the insta snapshot only if the sample class names change (they shouldn't).

## Risks

None — output-only change. Class IDs and coordinates are numeric and need no escaping.
