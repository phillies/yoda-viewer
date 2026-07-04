# Render the Label Overlay as Inline SVG (drop the data-URI `<img>`)

> Source: optimizations-and-features.md §3.2 · Effort: M–L
> Depends on: benefits from performance/01 landing first
> Enables: correctness/04 option A (working selection animation), features/16 (vertex editing),
> per-element hover/tooltips · Supersedes: the separate `HitAreaShape` layer

## Problem

The overlay pipeline today: `render_labels_to_svg` builds an SVG string →
`render_overlay_data_uri` (`crates/yoda-ui/src/lib.rs:1561-1587`) wraps + percent-encodes it →
browser decodes a `data:image/svg+xml` URI into an `<img class="overlay-image">`. Costs:

- Three copies of the geometry per update (string, encoded string, decoded image).
- CSS animation inside `<img>`-SVG is unreliable (correctness/04).
- The overlay is opaque to the DOM, so a **second, parallel** SVG (`CanvasOverlay` +
  `HitAreaShape`, lib.rs:1295-1510) re-renders the same geometry as transparent shapes just
  for click handling. Two sources of truth for shape geometry.
- No per-element styling, hover states, tooltips, or devtools inspection.

## Design

One inline `<svg>` inside `.canvas`, rendered by Dioxus, replacing both the overlay `<img>`
and the hit-area layer. `CanvasOverlay` grows into the single overlay component:

```
svg (viewBox = image dims, absolute inset-0, preserveAspectRatio xMinYMin meet)
├── for each visible label:
│   ├── polygon/rect fill+stroke   (visual, from current render_segmask/render_bbox styling)
│   └── text + rect label chip     (when show_class_id/name)
├── draw-mode layer (existing: capture rect, committed polygon, preview lines, vertices)
└── selection styling via class="selected" + APP_CSS rules (incl. marchingAnts keyframes)
```

### Componentization

- New `LabelShape` component in `yoda-ui`: props `{ label, color, selected, show_bbox,
  show_segmask, show_class_id, show_class_name, label_scale, editable }`. `editable` gates
  `pointer-events` + the `ondoubleclick` select handler (merging today's `HitAreaShape`).
- Port the exact visual attributes from `render_segmask` / `render_bbox` /
  `render_text_label` (`crates/yoda-core/src/render.rs:115-182`) so pixels don't change.
  Text needs no manual XML-escaping in rsx (Dioxus escapes text nodes) — but keep
  correctness/03 for the remaining string paths.

### What happens to `yoda-core::render`?

Keep it. Consumers that still need string SVG: the SSR fallback viewer
(`yoda-web/src/lib.rs:586-603`) and any future export/PNG-snapshot feature. The insta snapshot
test remains the spec for visual attributes; add a comment cross-linking `LabelShape` so the
two stay in sync. (If infra/02 removes the fallback's overlay, `render_labels_to_svg` becomes
export-only.)

### Interaction notes

- `pointer-events`: visual shapes in locked mode get `pointer-events: none` so pan/drag on the
  container keeps working; in unlocked+edit mode shapes get `pointer-events: all` +
  `cursor: pointer` and own dblclick-select (replacing `HitAreaShape` semantics exactly,
  including `stop_propagation`).
- z-order: image `<img>` first, svg second — unchanged from today's stacking.
- The JS pan/zoom transform applies to `.canvas-stage`, which contains both — no change.

## Migration steps

1. Build `LabelShape` + integrate into `CanvasOverlay` behind a temporary bool
   (`const INLINE_OVERLAY: bool`) for easy A/B during review.
2. Wire visibility, selection, and label-chip options; verify against the insta snapshot
   attribute-by-attribute.
3. Delete `HitAreaShape`, the `overlay-image` `<img>` path, and `render_overlay_data_uri`.
4. Add `@keyframes marchingAnts` to `APP_CSS`; hook `.selected` class (correctness/04-A).
5. Remove the now-unused `urlencoding` usage in `yoda-ui` (stays in `yoda-web`).

## Performance expectations

Dioxus diffs the vDOM: toggling `show_bbox` patches attributes instead of re-decoding an
image. For very dense images (>5k polygons) vDOM size grows; mitigate by keeping `points`
strings memoized per label (they only change when labels change — compute in a
`use_memo` keyed on `current_labels`). If profiling shows rsx overhead at extreme densities,
fall back to `dangerous_inner_html` on a `<g>` fed by `render_labels_to_svg` — still inline
DOM, one string copy, no percent-encoding.

## Testing

- Visual: before/after screenshots on example dataset (all four toggles, selection on/off).
- Interaction: dblclick-select on polygon & bbox, draw mode unaffected, pan/zoom unaffected.
- Snapshot: keep `overlay_render` insta test green (string renderer untouched).
- Add infra/03's E2E script step: toggle bbox → assert `svg rect` count.

## Risks

- Dioxus SVG attribute naming (`stroke-width` vs `stroke_width`) — the codebase already uses
  both forms correctly in `CanvasOverlay`/`TreeNodeIcon`; follow those.
- Subtle stacking/pointer-events regressions in draw mode — the capture rect must stay
  topmost within the draw layer.
