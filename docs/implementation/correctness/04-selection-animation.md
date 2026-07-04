# Fix the Dead `marchingAnts` Selection Animation

> Source: optimizations-and-features.md §2.2 · Effort: XS
> Depends on: decision in performance/02 (inline SVG overlay)

## Problem

The selected-polygon markup emitted by `render_segmask` (`crates/yoda-core/src/render.rs:125`)
sets `style="animation: marchingAnts 0.6s linear infinite;"`, but no
`@keyframes marchingAnts` is defined in `APP_CSS` (`yoda-ui/src/lib.rs:239`), `FALLBACK_CSS`
(`yoda-web/src/lib.rs:25`), or the generated SVG itself. The selection highlight silently
degrades to a static dashed outline. Additionally, even with keyframes, CSS animation inside
an `<img src="data:image/svg+xml…">` is not reliably animated across browsers.

## Options

**A. If performance/02 (inline SVG overlay) lands first — preferred**
The overlay becomes real DOM; define once in `APP_CSS`:

```css
@keyframes marchingAnts { to { stroke-dashoffset: -22; } }
```

and change the polygon style to animate `stroke-dashoffset` (the current `stroke-dasharray`
is `8,3`, so a period of `-(8+3)*2 = -22` gives a clean loop).

**B. Keep the data-URI overlay for now**
Embed the keyframes in the wrapper SVG so the animation lives inside the image document.
`render_overlay_data_uri` / `build_overlay_data_uri` gain:

```
<svg …><style>@keyframes marchingAnts{to{stroke-dashoffset:-22;}}</style>{inner}</svg>
```

Verify in Chromium + Firefox; if `<img>`-embedded SMIL/CSS animation is rasterized statically
(Safari does this), fall back to **C**.

**C. Drop the animation**
Delete the `style` attribute in `render_segmask` and rely on the existing
white-stroke + dashed + higher-opacity styling for selection. Zero risk, slight UX loss.

## Recommendation

Do **C** immediately (one-line cleanup, removes a lie from the code), then get the real
animation back as part of performance/02 via **A**.

## Testing

- Update `render.rs` tests `selected_index_none_no_highlight` (already asserts absence of
  `animation: marchingAnts`) and the `overlay_snapshot` insta snapshot.
- Manual: select an object in the browser, confirm visible motion (option A/B) or crisp static
  highlight (option C).
