# Persist Visibility Preferences Across Reloads

> Source: next-features.md §1.9 · Effort: S
> Depends on: features/04 (storage helper + dataset id)

## Problem

Per-class visibility (`hidden_classes`) and per-object visibility (`LabelObject.visible`)
reset on every page reload / image change respectively. Reviewing large label sets with a
noisy class hidden means re-hiding it constantly.

## Scope decision

Persist **class-level** visibility and **display toggles**; deliberately do *not* persist
per-object visibility:

- Per-class hiding is a session-spanning preference ("I never want to see `background`").
- Per-object hiding is ephemeral triage ("hide this one so I can see under it") — persisting
  it would require keying by image+index, goes stale the moment labels are edited externally,
  and surprising invisible objects are worse than re-clicking an eye icon.
- Display toggles (`show_bbox`, `show_segmask`, `show_class_id`, `show_class_name`) and the
  class filter (`filter_classes`, `filter_mode`) are the same category of preference — persist
  them in the same pass; restoring the filter also restores the working set after reload.

## Design

### Storage shape

One JSON blob per dataset under key `yoda.prefs.<dataset_id>`:

```json
{ "hidden_classes": [3, 7], "show_bbox": true, "show_segmask": true,
  "show_class_id": false, "show_class_name": true,
  "filter_classes": [1], "filter_mode": "Any", "v": 1 }
```

`serde` struct `UiPrefs` in `yoda-ui` with `#[serde(default)]` on every field (forward
compatible); version field for future invalidation.

### Write path

A `use_effect` watching the relevant `AppState` fields serializes + `local_set` (features/04
helper). Debouncing is unnecessary — these change at click frequency. To avoid the effect
firing per unrelated state change after performance/01's refactor, derive a
`prefs_snapshot: Memo<UiPrefs>` and effect on that (memo equality gates writes).

### Read path

On startup, after `dataset_id` is known and before/alongside the class-map fetch: parse the
blob, dispatch a new `AppAction::PrefsLoaded(UiPrefs)` that overwrites the corresponding
fields. Unknown class ids in `hidden_classes`/`filter_classes` (dataset changed) are
harmless — they simply never match; prune them opportunistically when the class index loads.

### Desktop

Same `localStorage` inside the webview (features/04 verified persistence); no extra work.

## Testing

- Reducer: `PrefsLoaded` applies fields; defaults untouched when blob absent.
- Serde roundtrip incl. missing-field defaults and unknown extra fields.
- E2E: hide a class + enable bbox → reload → both restored.

## Risks

None significant. Keep `UiPrefs` out of `AppState` serialization concerns — it's a projection,
constructed at the edges.
