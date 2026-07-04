# Persist and Restore the Last-Opened Image

> Source: next-features.md §2.5 (parity-checklist requirement) · Effort: S
> Depends on: features/03 (auto-scroll makes the restore visible)

## Design

Two complementary mechanisms; implement both (they serve different intents):

### 1. URL as shareable state (primary)

Sync `?image_path=<dataset-relative>` in the address bar:

- On selection change: `history.replaceState` via `document::eval` (replace, not push — image
  paging shouldn't pollute back-button history) or `web_sys::window().history()`. Guard with
  `#[cfg(target_arch = "wasm32")]`; the codebase already cfg-gates `web-sys` usage in
  `build_endpoint` (`crates/yoda-ui/src/lib.rs:1596-1611`).
- On startup: read `window.location.search` before the initial fetches in the first
  `use_effect` (lib.rs:598); if present and valid, `selected_image_path.set(Some(path))`.
- Note the SSR fallback already used the identical parameter name (`ViewerQuery.image_path`) —
  keep the convention even after infra/02 removes the fallback.
- Desktop: `web_sys` is unavailable; skip URL sync (cfg-gated), rely on mechanism 2.

### 2. `localStorage` as session memory (fallback)

- Key: `yoda.lastImage.<hash>` where `<hash>` is a short hash of the dataset identity so two
  datasets served on the same origin/port don't collide. The client doesn't know the image
  root — expose it minimally: add `dataset_id: String` (e.g. first 8 hex of
  seahash/fnv of the canonical image root) to `TreeStatusResponse` or a new
  `/api/dataset-info`, fetched at startup. (This id is reused by features/14 visibility
  persistence — build it once.)
- Write on every selection change; read at startup **only if** no URL param.
- Desktop webview: `localStorage` works inside the webview; verify persistence across app
  restarts on Windows WebView2 (it does persist under the app's user-data dir by default).

### Validation on restore

The stored/URL path may have vanished. Restore flow: check membership in `flat_nodes` (path
lookup map from features/03) after the flat index loads; if absent, silently ignore (no error
banner — stale memory is normal). This ordering matters: set a `pending_restore:
Signal<Option<String>>`, resolve it in a `use_effect` that watches `flat_nodes` becoming
non-empty.

## Storage helper

Small module `yoda-ui/src/storage.rs`:

```rust
pub fn local_get(key: &str) -> Option<String>;   // no-op stubs off-wasm
pub fn local_set(key: &str, value: &str);
```

(also used by features/14). Uses `web_sys::window()?.local_storage()`. `web-sys` features:
add `"Storage"`, `"History"` to the workspace `web-sys` dep features (`Cargo.toml:56`).

## Testing

- Unit: restore-validation logic (pure fn over `flat_nodes` + candidate path).
- E2E (infra/03): select image2 → reload page → image2 is selected and its tree row visible.
- Manual desktop: restart app, image restored.

## Risks

- Percent-encoding of paths with spaces in the URL param — `urlencoding` is already used for
  the same values in API calls; reuse it for the query writer and decode on read.
