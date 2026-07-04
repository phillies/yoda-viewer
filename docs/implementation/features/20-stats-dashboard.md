# Dataset Statistics Dashboard (+ CSV/JSON export)

> Source: optimizations-and-features.md §6.2 + next-features.md §3.3, §3.8 · Effort: L
> Depends on: class index (v1 works from it); counts need features/15-v2 or features/11;
> per-object geometry stats need a label-file sweep (see tiers)

## Tiers — ship in order

### Tier 1: from data already in memory (S)

Server computes from `ClassIndex` + `FlatIndex` + class map:

- images total / labeled / unlabeled; orphan label count (features/08)
- per-class: image count (images containing the class)
- per-top-level-folder (≈ train/val/test) breakdown of the above
- class co-occurrence counts (pairs sharing an image)

`GET /api/stats` → one JSON document. Computation is O(entries × classes-per-image); compute
on demand, no caching in v1.

### Tier 2: instance-level (M — needs a full label sweep)

Per-class **instance** counts, objects-per-image histogram, polygon vertex counts, bbox/mask
area distributions (normalized area — no image decode needed since coords are normalized;
pixel areas would need dimensions per image — skip, normalized is more comparable anyway),
degenerate-geometry flags (zero-area, out-of-range coords — QA gold).
Requires reading every label file: run like the class-index build (features/07 progress
machinery; `spawn_blocking`; cache result keyed by label-file validators — natural fit for
features/11's DB, do Tier 2 after it or accept recompute-per-request with a "computing…"
state).

### UI

New route/view — first real second page. Options: Dioxus router (feature `router` already
enabled in workspace dep) with `/stats`, or a modal panel. **Router** — stats deserve a URL.

- Table-first presentation (sortable columns: class, images, instances, %) — tables beat
  charts for exactness and cost nothing. One simple SVG bar chart (per-class image counts)
  hand-rolled — no chart dependency for v1.
- Every count is a **link where possible**: class row → sets the class filter and navigates
  back to the tree; unlabeled count → unlabeled filter (features/08). This makes stats a
  navigation hub, not a report.
- Co-occurrence: render as a top-N list ("wheel + bumper: 312 images"), not a matrix, until
  someone asks.

### Export (next-features §3.8 folded in)

- `GET /api/stats?format=csv` → per-class table as CSV (`text/csv`,
  `Content-Disposition: attachment`).
- `GET /api/export/image-classes?filter=…` → current filtered image list as CSV/JSON
  (`path, class_ids`) — the "cross-reference with training logs" ask. Client builds the link
  from active filter state ("Export list" button in the filter bar).

## Testing

- Pure stats computation functions in a new `yoda-data::stats` module — table-driven unit
  tests against a small fixture index (counts, co-occurrence, folder splits).
- Axum: `/api/stats` JSON shape; CSV escaping (class names with commas/quotes — RFC 4180
  quoting; reuse nothing, write the 10-line quoter, test it).
- E2E: navigate to /stats, click a class row, assert tree filter chip active.

## Risks

- Router introduction touches app bootstrapping (`RootApp`) — keep the tree/viewer page as
  the index route so deep links and existing behavior are unchanged.
- Tier 2 on 500k images without the DB cache is minutes of IO — gate behind explicit
  "Compute detailed stats" button until features/11 lands.
