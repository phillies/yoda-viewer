# Desktop Packaging & First-Run Experience

> Source: next-features.md §3.10 · Effort: L (spread over small stages)
> Depends on: correctness/05 (error screen — prerequisite for GUI-launched app),
> bold/08 overlaps (single-binary serves the CLI audience; this serves the no-terminal
> audience — decide which audience is real before investing past Stage 1)

## Current state

`yoda-desktop` is a thin Dioxus-desktop shell: spawns the Axum backend on a thread, opens a
webview (`crates/yoda-desktop/src/main.rs`). Configuration is env-vars only — hostile to
double-click launching. No installer, no icon, no dataset picker.

## Stage 1 — usable without a terminal (do first, small)

1. **Dataset picker.** When required paths are missing/invalid (instead of correctness/05's
   error screen alone): a folder-picker screen — image root, label root, optional YAMLs.
   `rfd` crate (native dialogs, used widely with Dioxus/Tauri apps) invoked from the webview
   via a Dioxus event → native dialog on the main thread (check `rfd` + dioxus-desktop
   threading: use `AsyncFileDialog` from a spawned task).
2. **Config persistence.** Chosen paths → `directories::ProjectDirs` config file
   (`yoda/config.toml`, serde). Load order: env vars override file (env stays authoritative
   for scripted use). Recent-datasets list (last 5) on the picker screen.
3. **Backend restart on dataset switch.** Today settings are fixed at startup. Rework the
   backend thread to be restartable: `build_router` per dataset, `axum::serve` with a
   shutdown channel (`watch::channel`); switching datasets = signal shutdown, rebuild, rebind
   (same port). UI shows a brief "Loading dataset…" (reuses features/07 status).
4. **Window niceties.** Title = dataset name; app icon (Dioxus desktop `Config::with_icon`);
   sensible default window size.

## Stage 2 — installers

- **Windows**: `cargo-wix` (WiX MSI) — most maintained path; or `cargo-packager`
  (Tauri's spiritual sibling, supports NSIS + WiX + macOS dmg + deb/AppImage from one config)
  — **prefer `cargo-packager`** for one-config-all-platforms.
- **Linux**: AppImage + .deb via cargo-packager. WebKitGTK runtime dependency for
  dioxus-desktop — AppImage must bundle or document it (this is the perennial pain; test on
  a clean Ubuntu container).
- **macOS**: .app + dmg unsigned initially; signing/notarization only if there's a user base
  (paid cert + CI secrets — defer explicitly).
- CI (infra/01 extension): tag-triggered `release.yml` building all three, attaching to a
  GitHub Release. Version from workspace `Cargo.toml`.

## Stage 3 — polish (only on demand)

Tray icon with server URL + "open in browser" (the backend is a real web server — letting a
desktop user hand a LAN URL to a colleague is a genuinely nice trick), auto-update
(cargo-packager supports updaters; needs hosting + signing — defer), file-association for
dataset YAMLs.

## Testing

- Stage 1: manual matrix (fresh machine/VM per OS): double-click → picker → dataset loads;
  restart → remembered. Unit-test the config load/override ordering.
- Stage 2: CI builds artifacts; install smoke on Windows VM + Ubuntu container.

## Risks

- dioxus-desktop (wry/tao) packaging quirks per-OS are the bulk of the unknown effort —
  timebox Stage 2 and prefer documenting `cargo install`+`yoda serve` (bold/08) if it
  drags; both stages share Stage 1's work regardless.
- Restartable backend (Stage 1.3) touches `yoda-web`'s router lifecycle — coordinate with
  features/10's mutable `flat_index` change (same refactor territory).
