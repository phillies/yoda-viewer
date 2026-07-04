# Read-Only Deploy Mode (`YODA_READ_ONLY`) + Optional Shared Token

> Source: optimizations-and-features.md §5.5 · Effort: S (read-only) + S (token)
> Depends on: nothing · Urgent for any non-localhost deployment (Docker defaults to 0.0.0.0)

## Problem

`PUT /api/labels` is open to anyone who can reach the port. The Docker image sets
`YODA_HOST=0.0.0.0`; pointing it at a real dataset currently means anyone on the network can
rewrite labels. There is no way to deploy a browse-only instance.

## Part 1 — `YODA_READ_ONLY`

### Config

`YoDaSettings` gains `pub read_only: bool` (default false); parse `"1" | "true"`
case-insensitively in `from_env_iter` (`crates/yoda-config/src/lib.rs:65-96`); add override
field. Document in README env table.

### Server enforcement (the part that matters)

In `api_router`, when read-only:

- Replace the labels route: `get(load_labels)` only — or keep the PUT bound to a handler
  returning `403 {"code":"read_only","message":…}` (better: explicit error beats 405
  ambiguity). Same treatment for future mutating routes (features/10 rebuild POST is
  *allowed* — it mutates cache, not data; features/21 class-ops are blocked).
- Disable all disk writes as a belt-and-braces invariant: class-index cache save
  (`save_to_disk` calls in `load_or_build` and the save handler) becomes in-memory-only.
  This also fixes read-only *filesystem* deployments (label dir mounted `ro`) where the cache
  write currently fails every startup — pass `read_only` into `ClassIndex::load_or_build` or
  gate at call sites.

### Client awareness

Expose it: extend `/api/health` (or the `dataset-info` endpoint from features/04) with
`"read_only": true`. UI on startup: force `AccessMode::Locked`, hide the "Unlock Editing"
button, show a `Read-only` status pill (reuse `.status-pill.locked` styling). Server remains
the enforcement point; UI is convenience.

## Part 2 — `YODA_AUTH_TOKEN` (optional, separate PR)

Shared-secret for mutating routes only (view stays open; full auth is out of scope):

- Middleware on the API router: if configured, mutating methods require
  `Authorization: Bearer <token>`; constant-time comparison (`subtle` crate or
  `ring::constant_time`).
- Client: token entry field in a small settings popover; store in `localStorage`
  (features/04's storage helper); attach header in `save_labels` /
  future mutating fetches. 401 → surface via features/09's `Failed` state with a
  "set token" hint.
- Explicitly document: token over plain HTTP is snoopable; pair with a reverse proxy for TLS.
  This is a tripwire, not a vault.

## Testing

- Axum tests: read-only router → GETs 200, PUT 403 with `read_only` code; no
  `.yoda_class_index.json` created in a fresh tempdir dataset.
- Token: PUT without header 401, with wrong token 401, with correct token 200; GET without
  token 200.
- UI manual: read-only instance shows pill, no unlock button.

## Risks

- Ordering with performance/04/features/11: the "no disk writes" invariant must be re-checked
  when the flusher/DB lands (grep for write paths; add a debug assertion in `write_atomic`
  helper when a global read-only flag is set — cheap safety net).
