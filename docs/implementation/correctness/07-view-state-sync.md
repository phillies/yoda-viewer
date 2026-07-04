# View State: Single Source of Truth + Always-Available Reset

> Source: next-features.md §1.1, §1.3, §2.3 · Effort: M
> Depends on: nothing · Related: correctness/08 (close-zone scaling) consumes the synced zoom

## Problem

Three intertwined issues:

1. **Divergence** (§1.1): `AppState.view: ViewTransform` and the reducer actions `SetZoom`,
   `ZoomBy`, `SetPan`, `PanBy`, `ResetView` (`crates/yoda-app/src/lib.rs:415-436`) exist, but
   the JS `PAN_ZOOM_SCRIPT` controller (`crates/yoda-ui/src/lib.rs:14-212`) is authoritative at
   runtime and never syncs back. Rust-side logic reading `state.view` gets stale values.
2. **No reset while editing** (§1.3): the JS `onDoubleClick` handler skips `resetView()` when
   `data-edit-mode === 'unlocked'`, and no alternative gesture exists.
3. **No fit/reset affordance** (§2.3): `resetView()` exists in JS but is only reachable via
   double-click in locked mode.

## Design decision

Do **not** try to mirror continuous pan/zoom into Dioxus state — every wheel tick would trigger
a Rust re-render for no benefit. Instead:

- **Demote** `AppState.view` to "JS-owned"; keep only what Rust genuinely needs: the current
  zoom factor, updated *coarsely* (see below) for hit-zone scaling.
- **Promote** reset/fit to a first-class command that works in every mode.

## Implementation

1. **Remove dead reducer surface.** Delete `SetZoom`, `ZoomBy`, `SetPan`, `PanBy` actions and
   the `pan_x`/`pan_y` fields. Keep `ResetView` (it also resets the JS side, step 3) and
   replace `ViewTransform` with a single `pub current_zoom: f32` on `AppState`.
2. **Coarse zoom sync JS → Rust.** The controller already writes
   `container.dataset.zoom = zoom.toFixed(2)` in `applyTransform`. Add a hidden input the JS
   updates and "changes" only when zoom crosses a 10% band boundary
   (`Math.round(zoom * 10)` changes), then dispatches an `input` event; Dioxus binds
   `oninput` on that element and dispatches a new `AppAction::ZoomChanged(f32)`. Banding keeps
   re-renders rare. (Same hidden-element bridge pattern as `DRAW_SCRIPT`'s key buttons; if/when
   features/02 migrates shortcuts to Dioxus-native events, migrate this too.)
3. **Reset command path Rust → JS.** Expose `window.__yodaPanZoomController.resetView` (it's
   already on the controller closure — hoist it onto the controller object). Dioxus can't call
   JS directly without `document::eval`; use `document::eval("window.__yodaPanZoomController?.resetView()")`
   inside the `ResetView` action's effect handling (add a new
   `AppEffect::ResetJsView` emitted by the reducer, executed in `run_effects`).
4. **UI affordances.** Toolbar button "Fit" (always enabled) dispatching `ResetView`; keyboard
   `F` via features/02. Restore double-click reset in unlocked mode *when the target is not a
   label hit-area* — the hit-area `ondoubleclick` handlers already call `stop_propagation()`
   (`yoda-ui/src/lib.rs:1483, 1502`), so simply deleting the `editMode === 'unlocked'` early
   return in the JS `onDoubleClick` gives: dblclick on object = select, dblclick on background
   = reset. Verify propagation ordering (the JS listener is on the container, Dioxus's on the
   SVG child — child runs first and stops propagation ✓).

## Testing

- Reducer tests: `ZoomChanged` updates `current_zoom`; `ResetView` emits `ResetJsView` effect
  and sets `current_zoom = 1.0`; removed actions no longer compile (compile-time check).
- Manual matrix: reset via button / `F` / dblclick-background in locked and unlocked modes;
  dblclick on a polygon still selects without resetting.

## Risks

- `document::eval` availability differs between web and desktop renderers in Dioxus 0.7 —
  verify on both; fall back to a hidden-button-with-JS-listener bridge (JS listens for clicks
  on `#yoda-view-reset`) if eval misbehaves in the desktop webview.
