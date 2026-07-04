# Zoom-Aware Polygon Close-Zone Radius

> Source: next-features.md §1.2 · Effort: S
> Depends on: correctness/07 (coarse zoom sync provides `AppState.current_zoom`)

## Problem

`CLOSE_ZONE_RADIUS = 18.0` (`crates/yoda-ui/src/lib.rs:330`) is expressed in **image-pixel
space**. The draw overlay SVG uses `viewBox = image dimensions`, so the on-screen size of the
close zone scales with both the image's CSS fit *and* the JS pan/zoom transform. On a 4000px
image displayed at fit-to-screen, 18 image-pixels can be ~3 screen-pixels — effectively
unclickable; zoomed to 6×, it covers a huge area and steals clicks meant to place nearby
vertices.

Affected logic: `cursor_near_close` and the close test in the draw `onclick`
(`yoda-ui/src/lib.rs:1319-1325, 1380-1388`), plus the rendered indicator ring
(`r: "{CLOSE_ZONE_RADIUS}"`, line 1434).

## Design

Target: the close zone should be a constant **screen-space** size (~14 px radius). Convert to
image space per interaction:

```
effective_radius_image_px = TARGET_SCREEN_RADIUS / total_scale
total_scale = css_fit_scale * js_zoom
```

- `js_zoom`: from `AppState.current_zoom` (correctness/07). If 07 isn't done yet, read the
  container's `data-zoom` attribute — but 07 is the clean path.
- `css_fit_scale`: rendered image width ÷ natural width. The overlay `<svg>` has the same
  rendered size as the image; obtain it via the mouse event's element geometry. Dioxus'
  `element_coordinates()` is already in SVG-viewBox units — which means the *click position*
  handles CSS fit automatically; only the **radius comparison** needs the scale. Compute it
  as `event.element_coordinates() / event.client_coordinates()` deltas is fragile; instead use
  `onmounted`/`getBoundingClientRect` via `element.get_client_rect()` on the SVG once per
  image load and store `css_fit_scale` in a signal, recomputed on window resize (a `resize`
  listener in `DRAW_SCRIPT` clicking a hidden refresh button, or accept staleness until next
  image load — acceptable v1).

Implementation:

1. New helper in `yoda-ui`:
   `fn close_zone_radius(css_fit_scale: f32, js_zoom: f32) -> f32` with
   `const TARGET_SCREEN_RADIUS: f32 = 14.0;` and clamping to
   `[4.0, 64.0]` image-px so degenerate scales stay usable.
2. `CanvasOverlay` gains a `close_radius: f32` prop computed in `App` from the two signals;
   replace all three uses of `CLOSE_ZONE_RADIUS` (hover test uses `* 2.0` today — keep the
   more forgiving hover multiplier but derive both from the prop).
3. The indicator ring `r` uses the same prop so visual size matches hit size.

## Testing

- Unit-test `close_zone_radius` (pure function): fit-scale 0.25 & zoom 1 → 56 (clamped 56<64 ok);
  zoom 6 → small; clamps hold.
- Manual: draw + close a polygon on the example dataset at min zoom, fit, and 6× zoom; the
  ring should look the same size on screen in all three.

## Risks

- `get_client_rect` timing: on first render the image may not be laid out; guard with a
  fallback of `css_fit_scale = 1.0` and recompute on the first `onmousemove` (cheap: only when
  signal is `None`).
