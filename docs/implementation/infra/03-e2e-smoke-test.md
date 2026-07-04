# End-to-End Smoke Test (Playwright against the WASM bundle)

> Source: optimizations-and-features.md §4 · Effort: M
> Depends on: infra/01 (CI) for automation; runnable locally without it

## Problem

Nothing exercises the hydrated WASM app. Unit tests cover the reducer, parsers, and Axum
handlers, but the classic failure mode of this architecture — SSR/API fine, hydration or
fetch-glue broken — ships silently. One browser-driven smoke test catches the whole class.

## Design

Keep it deliberately small: **one spec file, one happy path, no test pyramid.**

### Fixture dataset

`tests/e2e/fixture/` checked into the repo (tiny): 2 images (64×64 PNG, generated once, a few
hundred bytes each), matching label files (1 polygon + 1 bbox), a 3-class `classes.yaml`, and
a `color_map.yaml`. Deterministic and license-free (generate with the `image` crate via a
small `cargo xtask gen-fixture` or a checked-in script).

### Server under test

Script `tests/e2e/run.sh` (and CI steps):

1. `dx build --release -p yoda-web` (or reuse CI's `web-build` artifact).
2. Launch the built server with env pointing at the fixture,
   `YODA_PORT=8123 YODA_HOST=127.0.0.1`, labels copied to a temp dir (the test **edits** them).
3. Wait for `/api/health`.

### The spec (`tests/e2e/smoke.spec.ts`)

```text
1. goto / → expect tree row for fixture folder; expand; click image1.png
2. expect main image visible; expect overlay svg/img present
3. toggle "BBox" → overlay updates (assert element/attr change)
4. click "Unlock Editing" → status pill shows Unlocked
5. change class dropdown on object #1 → expect "Labels saved" status
6. assert on-disk: temp label file line now starts with the new class id
7. draw path (stretch, optional v2): enter Draw, click 3 points + Enter, expect object count +1
```

Step 6 crosses from browser to filesystem: do the assertion in the Playwright test via a tiny
`/api/labels` GET (stays in-band, no fs access needed from the test runner — cleaner).

### Tooling choice

Playwright (`npm` dev-dependency confined to `tests/e2e/package.json`). Alternative considered:
`fantoccini`/WebDriver from Rust — keeps the repo npm-free but is far more code for worse
diagnostics (no trace viewer). Playwright's failure artifacts (screenshot, trace) are the
main reason to pick it.

### CI wiring

Fourth job in `.github/workflows/ci.yml`: needs `web-build` artifacts (upload
`target/release/yoda-web` + `public/` from that job), `npx playwright install chromium
--with-deps`, run spec, upload trace on failure. Non-required check for the first weeks, then
promote.

## Selector hygiene

Add `data-testid` attributes where selectors would otherwise be brittle: tree rows
(`data-testid="tree-row"` + name), toolbar toggles, status pill, object rows. Cheap now,
saves every future spec.

## Testing the test

- Intentionally break hydration locally (serve without `public/`) → spec must fail at step 1
  (after infra/02, it fails on the setup page marker — even clearer).
- Flake policy: retries=1 in CI, and any flake gets a linked issue rather than a silent retry
  bump.

## Risks

- `dx build` in CI adds minutes — mitigated by reusing the `web-build` job's output.
- Desktop app remains untested E2E; out of scope (webview automation is poor ROI; the shared
  `yoda-ui` code means web coverage covers most of it).
