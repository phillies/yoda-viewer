# Multi-User Review Workflow (statuses, notes, presence)

> Source: optimizations-and-features.md §7.4 · Effort: XL (staged)
> Depends on: features/11 (SQLite — hard prerequisite), features/12 (token — recommended),
> features/10 v2 SSE groundwork

## Concept

Per-image review status (`unreviewed / approved / needs_fix`), reviewer notes, an audit trail
of label edits, and lightweight presence — enough for a two-to-five-person labeling team
without accounts infrastructure. Identity = self-declared display name (cookie/localStorage),
not authentication; features/12's shared token gates write access.

## Stage 1 — review status + notes (the 80%)

### Schema (extends features/11's DB; these tables are *workflow*, not cache — they get real
migrations from day one)

```sql
CREATE TABLE review (
  image_path TEXT PRIMARY KEY,
  status TEXT NOT NULL DEFAULT 'unreviewed',   -- unreviewed|approved|needs_fix
  note TEXT,
  updated_by TEXT, updated_at INTEGER
);
CREATE TABLE audit (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  image_path TEXT, actor TEXT, at INTEGER,
  kind TEXT,            -- save_labels|status_change|class_op
  detail TEXT           -- JSON: label count delta, old/new status, …
);
```

### API

```
GET  /api/review?image_path=…       PUT /api/review  {image_path, status, note}
GET  /api/review/summary            → counts per status (+ per top-level folder)
GET  /api/audit?image_path=…&limit=…
```

Actor from a `X-Yoda-User` header the client always sends (name from a first-run prompt,
stored via features/04's storage helper). `save_labels` handler writes an audit row.

### Client

- Status control in the toolbar (three-state segmented button) + note field in the right
  panel; keyboard `A` approve / `X` needs-fix (features/02 map).
- Tree: status glyph per image row (○ / ✓ / ⚠, colored); filter chips per status (composes
  via the generalized predicate); review-summary line in the filter bar ("1 204 approved /
  310 needs fix / 2 486 unreviewed").
- **Review queue flow**: "next unreviewed" button = features/01 stepping over the
  status-filtered sequence. This turns YoDa into a review conveyor with zero extra machinery.

## Stage 2 — freshness + presence

- SSE endpoint `/api/events` (axum `Sse` + `tokio::sync::broadcast`): events
  `review_changed {path}`, `labels_changed {path}`, `presence {user, path}`.
- Client: subscribe on load; patch review state live; **conflict warning**: if
  `labels_changed` arrives for the currently-open image from another actor, banner
  "wheel@anna saved changes to this image — reload / keep mine (last write wins)".
  True merging is out of scope, permanently (say so in the UI copy).
- Presence: client POSTs heartbeat (`/api/presence {path}`) every 15 s; server broadcasts;
  tree rows show a small avatar-dot for images someone else has open. Ephemeral, in-memory.

## Stage 3 — assignment (only if a real team asks)

`assignment` table (image_path → assignee), bulk-assign from the filtered set, "my queue"
filter. Deliberately last: folders-as-assignments is the zero-code workaround teams already
use.

## Testing

- Stage 1: Axum tests for review CRUD + audit rows on save; DB migration test (open v1 db,
  migrate, data intact). Reducer/UI: status filter predicate.
- Stage 2: SSE integration test (subscribe, trigger save from second client, assert event);
  conflict banner logic unit test (actor comparison).

## Risks

- Identity is honor-system — fine for the target team size; document loudly. Real auth is a
  different product tier; don't half-build it.
- SSE through proxies (buffering) — document `X-Accel-Buffering: no` header and keepalive
  comments (axum `Sse::keep_alive`).
- Scope gravity toward "CVAT clone" — the counter-position: no accounts, no roles, no task
  management beyond Stage 3. Anything more belongs in a dedicated tool.
