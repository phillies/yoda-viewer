# Dataset-Wide Class Operations (rename / merge / delete / renumber)

> Source: optimizations-and-features.md §6.3 · Effort: L
> Depends on: features/12 (must respect read-only), features/10 (index refresh after),
> strongly recommend features/11 (validators make the post-op resync cheap)
> This mutates every label file — the highest-stakes feature in the near/mid set.

## Operations

| Op | Label files | YAML `names:` |
|----|-------------|----------------|
| Rename class | untouched | value changed |
| Merge A→B | every line with A rewritten to B | A removed |
| Delete class | every line with A **removed** | A removed |
| Renumber compact | all ids remapped to 0..n dense | keys remapped |

All are expressible as one primitive: `remap: HashMap<u32, Option<u32>>`
(Some = new id, None = drop lines) + a YAML edit. Build the primitive, expose the four
operations as UI sugar over it.

## Design

### Core (`yoda-core` / `yoda-data`)

```rust
/// Rewrite one label file's class ids. Returns lines changed.
pub fn remap_label_file(path: &Path, remap: &HashMap<u32, Option<u32>>) -> Result<usize, LabelError>
```

- Operates on **text lines** (first token swap / line drop), not parsed `LabelObject`s —
  avoids float reformatting of untouched coordinate data (byte-preserving for unaffected
  lines; only the class token changes). This matters: a full parse/serialize would dirty
  every line's float formatting and destroy diffability.
- Atomic write per file (correctness/02 helper).

### Orchestration (`yoda-web`)

Two-phase, dry-run-first API:

```
POST /api/class-ops/plan    { remap: {...}, yaml_edit: {...} }
  → { affected_files: N, affected_lines: M, sample: [first 20 paths], plan_id }
POST /api/class-ops/apply   { plan_id }
  → 202 + progress via the features/07-style status endpoint
```

- `plan` computes from the class index (which files contain remapped ids) — instant, no file
  reads. `plan_id` = hash of the remap + index generation; `apply` re-validates the index
  hasn't changed since planning (else 409, re-plan).
- `apply` runs in `spawn_blocking` with progress; **backup first**: copy affected label files
  to `<label_root>/.yoda/backup/<timestamp>/<relative-path>` before rewriting (bounded by
  affected set, not whole dataset). A `class-ops/rollback` endpoint restoring the newest
  backup set is cheap insurance and turns "scary" into "routine".
- YAML edit: parse the dataset YAML (`serde_yaml`), rewrite `names:` preserving the rest of
  the document (Ultralytics YAMLs carry `path/train/val` keys — round-trip via `Value`, edit
  the mapping in place, write atomically). If no YAML configured, skip with a warning in the
  plan response.
- Post-apply: patch the class index in memory using the same remap (no full rebuild needed —
  entries are id-lists), mark dirty; connected-client staleness handled as in features/10.

### UI

Management panel reachable from the filter bar (or the stats page, features/20): pick
operation, pick classes (source/target dropdowns from the union of class map +
`all_class_ids` — correctness/09), see the plan ("would modify 1 243 files / 5 019 lines,
backup will be created"), type-to-confirm (`merge` literal or the class name) for
delete/merge, then a progress bar. Refuse while unlocked-with-unsaved-failed-save
(features/09 state) or read-only.

## Testing

- `remap_label_file` unit: swap, drop, untouched-line **byte identity** (assert exact bytes
  of unaffected lines), empty-result file (all lines dropped → empty file, not deleted).
- Plan/apply Axum test on a fixture: counts correct; backup exists and matches pre-state;
  409 on stale plan (edit a label between plan and apply); rollback restores.
- YAML edit roundtrip preserves unrelated keys and key order (serde_yaml mapping order —
  verify; if it reorders, switch to targeted line editing for the names block or accept
  reordering with a documented note).

## Risks

- Highest data-risk feature here; the backup + dry-run + type-to-confirm trio is the design,
  not decoration. Do not ship any of the four ops without all three.
- Concurrent external edits during apply: file-level last-writer-wins; the backup covers
  recovery. Document "don't run training-set writers during class ops".
