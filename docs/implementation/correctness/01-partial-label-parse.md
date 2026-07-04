# Partial Label Parsing (stop dropping whole files on one bad line)

> Source: optimizations-and-features.md §2.4 · Effort: S · Priority: **highest**
> Depends on: nothing · Enables: save-status warnings (features/09)

## Problem

`parse_yolo_labels` (`crates/yoda-core/src/label.rs:76-91`) maps any `LabelError` from
`parse_yolo_label_text` to an empty `Vec` via `unwrap_or_default()`. One malformed line makes
an image with hundreds of valid objects appear unlabeled. Worse: if the user then performs any
edit (class change, delete, draw), `save_effect_for_current_image` persists the now-empty/partial
`current_labels`, **erasing the valid lines on disk**.

## Design

Parse line-by-line and make partial success the normal result:

```rust
pub struct ParseOutcome {
    pub labels: Vec<LabelObject>,
    /// One entry per skipped line: (line_number, reason)
    pub skipped: Vec<(usize, String)>,
}

pub fn parse_yolo_labels_lossy(path, w, h) -> ParseOutcome
```

- Keep `parse_yolo_label_text` (strict, `Result`) for tests and for callers that want
  all-or-nothing; implement it on top of the lossy version (error if `skipped` non-empty).
- In the lossy loop, on a per-line error push `(line_number, error.to_string())` into
  `skipped` and continue. `index` values must stay sequential over *kept* labels only
  (assign `labels.len()` at push time instead of `line_index` — note `parse_bbox`/`parse_polygon`
  currently receive `line_index` as the object index; change the call sites to pass a running
  counter).
- Dimension validation failure (`InvalidImageDimensions`) stays a hard error — nothing can be
  parsed without dimensions.

## Propagation

1. `yoda-data` `DatasetRepository::load_labels` → return `ParseOutcome` (or add
   `load_labels_with_warnings`; keep the old signature delegating for a soft migration).
2. `yoda-app` `LoadedImage` gains `pub warnings: Vec<String>` (formatted
   `"line 17: invalid coordinate \"abc\""`). `AppAction::ImageLoaded` copies warnings into
   `state.status.error_text` (joined, truncated to ~3 entries + "…and N more").
3. `yoda-web` `LabelsResponse` gains `pub warnings: Vec<String>` (serde default so old clients
   don't break). Populate in `load_labels` and `save_labels` handlers
   (`crates/yoda-web/src/lib.rs:449, 468`).
4. `yoda-ui` `LabelsResponse` mirror struct (`crates/yoda-ui/src/lib.rs:332`) gains
   `#[serde(default)] warnings`.

## Safety interlock

Until warnings are surfaced everywhere, add a guard in the reducer: if
`current_labels` was loaded with warnings, the first mutating action shows a confirm-style
error (`"This file has N unparsed lines; edits will rewrite the file without them."`) via
`status.error_text` and requires a second invocation (or simply block save when
`warnings > 0 && !state.acknowledged_warnings`). Keep the interlock simple — a
`bool acknowledged_warnings` on `AppState` reset on `ImageLoaded`.

## Testing

- Unit (`yoda-core`): file with `good\nbad\ngood` → 2 labels, 1 skipped with line number 2;
  indices are `0,1`; empty + missing file → empty outcome, no warnings; all-bad file → 0 labels,
  N warnings.
- Roundtrip: load lossy file, save → file now contains only the 2 valid lines (documented,
  intentional after acknowledgment).
- Axum test (`yoda-web`): `GET /api/labels` on a corrupt fixture returns 200 with
  `warnings.len() == 1`.

## Risks

- Behavior change: previously-empty-looking files will now show objects. That's the point, but
  mention it in the changelog.
- Any external consumer of `parse_yolo_labels`' current signature — grep shows only
  `yoda-data/src/lib.rs:160` and tests.
