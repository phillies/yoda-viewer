# Shrink the SSR Fallback + Remove Legacy Tree Routes

> Source: next-features.md §1.7, §1.8 + optimizations-and-features.md §4 (duplication)
> Effort: M (mostly deletion) · Depends on: decision below
> Related: bold/08 (single-binary) removes the "no assets" state entirely — if 08 is
> committed to, do only the route cleanup here.

## Problem

- When no Dioxus client assets exist next to the binary, `build_router`
  (`crates/yoda-web/src/lib.rs:273-291`) serves a hand-rolled server-rendered viewer:
  ~500 lines of parallel HTML/CSS/tree/legend/object-list rendering
  (`FALLBACK_CSS`, `render_fallback_viewer`, `build_fallback_view`, `render_fallback_html`,
  `render_tree_html`, `render_class_legend_html`, `render_object_list_html`, icon SVG dupes).
  It is read-only, has no real pan/zoom state, duplicates the theme (and has *diverged* —
  blue vs olive), and duplicates `dataset_relative_path` (also in `yoda-data`). Because
  `cargo run -p yoda-web` without a prior `dx build` lands here, it's many users' confusing
  first impression.
- `GET /api/tree` and `GET /api/tree/children` exist only for this fallback + old clients;
  the SPA uses `/api/tree/flat` exclusively (next-features §1.7). They keep the recursive
  `TreeNode` format and the `expand_directory` repository surface alive.

## Decision

Replace the fallback viewer with a **static instruction page**; delete the legacy routes.
Rationale: the fallback's only real job is telling you the web bundle is missing. A viewer
that silently lacks most features does that job worse than a page that says so.

## Implementation

1. **Instruction page.** New `render_setup_page()` returning a single small HTML page:
   what was detected (`public_dir` path from `log_public_assets_state`), what to run
   (`dx build` / `dx serve` / docker image), and a link to `/api/health` for backend sanity.
   Route `/` in the no-assets branch serves it; keep `/favicon.ico` no-content handler.
2. **Delete** `FALLBACK_CSS`, `FallbackViewerState`, `build_fallback_view`,
   `render_fallback_html`, `render_tree_html`, `render_tree_entry_html`,
   `render_class_legend_html`, `render_object_list_html`, `build_overlay_data_uri`,
   `sorted_entries`, `find_first_image`, `folder_icon_svg`, `image_icon_svg`, and the
   `ViewerQuery` type. Keep `escape_html`/`escape_attr` only until correctness/03 moves the
   shared escaper into `yoda-core` (coordinate).
3. **Delete routes** `/api/tree` + `/api/tree/children` and handlers `list_tree` /
   `list_children`; then remove `TreeNode`-producing surface from `yoda-data`:
   `list_root_nodes` / `expand_directory` from the `DatasetRepository` trait,
   `get_file_tree`, `get_dir_children`, `tree_node_for_path`, `LAZY_PLACEHOLDER_SUFFIX`,
   `NodeIcon::Placeholder` (Folder/Image icons stay — `yoda-ui` uses them). Migrate/delete the
   associated tests (`tree_structure`, `dir_gets_lazy_placeholder`, etc. — behavior now covered
   by `scan_tests`).
4. **API deprecation etiquette.** This is a self-contained app (client + server ship
   together); no external consumers are plausible. Note removal in the changelog; bump minor
   version.
5. **README.** Replace "fallback viewer" implications with the build-first flow; document
   `dx build` as the required step (next-features §3.6's doc half; the CI half lives in
   infra/01).

## Payoff

≈ 600–700 lines deleted across `yoda-web` and `yoda-data`, one theme, one tree
implementation, one escaping helper, and a first-run experience that explains itself.

## Testing

- Axum test: no-assets router serves `/` with the setup page (assert marker string), API
  routes still function, removed routes 404.
- `cargo test --workspace` for the trait-surface removal fallout.

## Risks

- Anyone scripting against `/api/tree` breaks — accepted (see step 4).
- If instead the team wants the fallback to remain a viewer, the alternative path is to
  *hydrate* it properly via bold/08 (embed assets) — do not invest in improving the parallel
  HTML renderer under any plan.
