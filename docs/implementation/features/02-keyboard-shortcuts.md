# Keyboard Shortcuts (full map + migration off hidden buttons)

> Source: next-features.md §1.4, §2.2 · Effort: M
> Depends on: features/01 (arrow-key nav targets), correctness/07 (`F` = reset view)

## Current state

`DRAW_SCRIPT` (`crates/yoda-ui/src/lib.rs:214-237`) wires exactly three keys
(Delete/Backspace, Escape, Enter) by `.click()`ing hidden Dioxus buttons
(`#yoda-key-delete|escape|enter`, lib.rs:897-920). The Python version's `M B I N F E D`
toggles are absent.

## Target key map

| Key | Action | Notes |
|-----|--------|-------|
| `M` | `ToggleSegmask` | |
| `B` | `ToggleBbox` | |
| `I` | `ToggleClassId` | |
| `N` | `ToggleClassName` | |
| `F` | `ResetView` | works in all modes (correctness/07) |
| `E` | `SetInteractionMode(Edit)` | only meaningful unlocked |
| `D` | `SetInteractionMode(Draw)` | reducer already blocks when locked with an error message ✓ |
| `L` | toggle `SetAccessMode` | lock/unlock — proposed addition |
| `←` / `→` | prev / next image | features/01 |
| `Delete`/`Backspace` | `DeleteSelectedLabel` | existing |
| `Escape` | `CancelDrawing` (+ clear selection when not drawing — new) | existing, extended |
| `Enter` | `FinishDrawing` | existing |
| `?` | show shortcut overlay | see below |

## Design: migrate to a Dioxus-native global handler

Replace the hidden-button bridge with a single `onkeydown` on the app root. Dioxus 0.7
supports keyboard events on focusable elements; a `div` wrapping `.app-shell` with
`tabindex: "0"` + autofocus receives keys, but focus is easily lost to buttons. Robust
approach that stays in Rust: keep **one** tiny JS relay that forwards *all* non-editable
keydowns as a `CustomEvent` → hidden `<input>` whose value is the key name, triggering
`oninput` — i.e. one bridge element instead of one per action:

```
JS: document.addEventListener('keydown', e => {
      if (tag is INPUT/SELECT/TEXTAREA) return;
      relay.value = serialize(e);           // "ArrowRight", "m", "M+shift", …
      relay.dispatchEvent(new Event('input', {bubbles:true}));
      if (HANDLED_KEYS.has(e.key)) e.preventDefault();
    });
Rust: oninput → match key string → dispatch AppAction
```

- All routing logic (mode-awareness, locked checks) lives in one Rust
  `fn action_for_key(key: &str, state: &AppState) -> Option<AppAction>` — unit-testable,
  which the hidden-button approach never was.
- Delete the three hidden buttons and their handlers.
- `preventDefault` only for keys we handle, so browser shortcuts (Ctrl+R etc.) survive; keep
  ignoring events with `ctrlKey/metaKey` modifiers except future undo (features/19 adds
  `Ctrl+Z` — design the serializer to include modifiers now: `"ctrl+z"`).

## Escape semantics

Today Escape always dispatches `CancelDrawing`. Extend `action_for_key`: in Draw mode →
`CancelDrawing`; else if `selected_object_index.is_some()` → `SelectLabel { label_index: None }`;
else no-op. Pure-Rust logic, trivially tested.

## Discoverability

- `?` opens a static overlay (simple absolutely-positioned panel listing the table above,
  closed by `?`/Escape/click). Also add `title` attributes to the corresponding toolbar
  buttons (`"Mask (M)"` …) — one-line changes, large usability win.

## Testing

- Unit: `action_for_key` matrix — locked vs unlocked, draw vs edit, selection vs none,
  input-focused exclusion happens JS-side (document assumption).
- E2E (infra/03): press `B` → bbox toggle asserted; `→` → image advances.

## Risks

- Key handling inside the desktop webview: verify the relay listener fires there (it's the
  same webview DOM — expected fine).
- International layouts: match on `event.key` (layout-aware character), not `event.code` —
  the serializer above already does.
