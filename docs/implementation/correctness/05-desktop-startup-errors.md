# Surface Desktop Backend Startup Failures in the Window

> Source: optimizations-and-features.md §2.5 · Effort: S
> Depends on: nothing

## Problem

`run_desktop` (`crates/yoda-desktop/src/main.rs:40-104`) spawns the Axum backend on a thread
and only logs failures there. `build_router` — which validates `YODA_IMAGE_BASE_PATH` etc. via
`BackendState::from_settings` → `canonical_dir` — runs *inside* that thread, after
`dioxus::launch` is already committed. A misconfigured path gives the user a window full of
failed fetches with the real cause buried in stderr (which is invisible when launched from a
desktop shortcut).

## Design

1. **Build the router before spawning.** `build_router(settings)` is synchronous. Restructure:

```rust
let router = match build_router(server_settings) {
    Ok(r) => r,
    Err(e) => { STARTUP_ERROR.set(format!("{e:?}")).ok(); /* still launch UI */ }
};
```

   Move the `TcpListener::bind` result into the same pre-launch phase (bind std listener,
   convert with `tokio::net::TcpListener::from_std` inside the runtime thread) so port
   conflicts are also caught up front.

2. **Error screen.** Add a `static STARTUP_ERROR: OnceLock<String>` next to
   `DESKTOP_API_BASE`. `desktop_app()` checks it first:

```rust
fn desktop_app() -> Element {
    if let Some(msg) = STARTUP_ERROR.get() {
        return rsx! { ConfigErrorScreen { message: msg.clone() } };
    }
    rsx! { App { api_base: DESKTOP_API_BASE.get().cloned() } }
}
```

   `ConfigErrorScreen` (new component in `yoda-ui`, reusing `.empty-state` styling) shows the
   message plus a short checklist of the required env vars (`YODA_IMAGE_BASE_PATH`,
   `YODA_LABEL_BASE_PATH`) and where they're read from (env / `.env`).

3. **Runtime death.** If the server thread dies later, the UI degrades to fetch errors as
   today. Optional hardening: `App` pings `/api/health` on mount and swaps in a
   "backend unreachable" banner (the fetch-error path in `set_status_error` already gets
   close; a dedicated message is friendlier).

## Testing

- Manual: `YODA_IMAGE_BASE_PATH=/nonexistent cargo run -p yoda-desktop` → window shows the
  config error with the offending path, not a broken viewer.
- Unit: none needed beyond compilation; `ApiError` from `build_router` already carries the
  path in its message (`canonical_dir` formats it).

## Risks

`ApiError` currently doesn't implement `Display` publicly with a friendly message (it's built
for HTTP). Either format via the existing `Debug` or add `impl Display for ApiError` returning
`self.message` — the latter is nicer and useful elsewhere.
