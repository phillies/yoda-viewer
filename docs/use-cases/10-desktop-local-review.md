# Use Case 10 — Review a Local Dataset with the Desktop App

> **Goal:** the full YoDa experience as a native window for datasets on your own machine —
> no browser tab, no separately managed server.
> **Actor:** anyone working with local datasets who has a Rust toolchain.
> **Mode:** deployment.

## Walkthrough

1. **Set the environment** (shell exports or a `.env` file in the working directory):

   ```bash
   export YODA_IMAGE_BASE_PATH="$HOME/datasets/carparts/images"
   export YODA_LABEL_BASE_PATH="$HOME/datasets/carparts/labels"
   export YODA_CLASS_INFO_YAML="$HOME/datasets/carparts/carparts-seg.yaml"   # optional
   ```

2. **Run:**

   ```bash
   cargo run -p yoda-desktop            # or a release build: cargo run --release -p yoda-desktop
   ```

3. A native window opens with the identical UI to the web app — same tree, filter, viewer,
   editing, shortcuts. Under the hood the app started its own backend on `127.0.0.1`,
   picking the first free port at or above `YODA_PORT` (default 8080) automatically, so port
   collisions with other tools resolve themselves.

## What's different from the web app

- Nothing functionally — `yoda-desktop` renders the same shared UI (`yoda-ui`) against the
  same backend (`yoda-web`'s router) it hosts itself.
- The backend is reachable at `http://127.0.0.1:<port>` while the app runs — handy for
  firing API calls (use case [11](11-scripting-with-the-api.md)) against the same session.
- Bound to localhost only; the desktop app is not a way to serve teammates (use
  [09](09-serve-a-remote-dataset.md) for that).

## Current limitations

- **Terminal-first UX.** There is no dataset picker, no installer, no app icon — env vars +
  `cargo run` is the interface ([packaging + first-run UX
  designed](../implementation/features/23-desktop-packaging.md)).
- **Misconfiguration is silent in the window.** If the image path is wrong, the backend
  thread fails but the window still opens and shows fetch errors; the real cause is only in
  the terminal log ([fix designed](../implementation/correctness/05-desktop-startup-errors.md)).
- Switching datasets means quitting, changing env vars, and relaunching.
- Session state (zoom, toggles, last image) resets per launch, same as the web app
  ([designed](../implementation/features/04-last-image-persistence.md),
  [14](../implementation/features/14-visibility-persistence.md)).

## Related use cases

- [01 — Browse and inspect](01-browse-and-inspect-a-dataset.md)
- [09 — Serve a remote dataset](09-serve-a-remote-dataset.md)
