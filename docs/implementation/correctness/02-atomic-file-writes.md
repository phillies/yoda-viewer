# Atomic Writes for Label Files and the Class-Index Cache

> Source: optimizations-and-features.md §2.3 · Effort: S
> Depends on: nothing

## Problem

- `write_yolo_labels` (`crates/yoda-core/src/label.rs:202-213`) calls `fs::write` directly.
  Every class change / delete / draw triggers an immediate save, so a crash or power loss
  mid-write truncates the user's label file on a routine edit.
- `ClassIndex::save_to_disk` (`crates/yoda-data/src/class_index.rs:85-89`) has the same
  pattern for `.yoda_class_index.json` (recoverable — it's a cache — but a corrupt JSON file
  currently fails `load_from_disk` silently and forces a full rescan).

## Design

Add one shared helper in `yoda-core` (new module `fs_util.rs`, re-exported from `lib.rs`):

```rust
/// Write atomically: write to a temp file in the same directory, fsync, rename over target.
pub fn write_atomic(path: &Path, contents: &[u8]) -> io::Result<()>
```

Implementation notes:

- Temp file must live **in the same directory** as the target so `rename` stays on one
  filesystem (`tempfile::NamedTempFile::new_in(parent)` + `persist(path)` does exactly this
  and handles cleanup on failure; `tempfile` is already a workspace dep, but currently
  dev-only — promote it to a regular dependency of `yoda-core`, or hand-roll with
  `{path}.tmp.{pid}` if avoiding the dep is preferred).
- Call `file.as_file().sync_all()` before persist for the durability guarantee; skip
  syncing the directory (overkill for this tool).
- Windows: `persist`/`rename` fails if the target is open elsewhere — acceptable; propagate
  the error (it surfaces via the existing save-error path).

## Call-site changes

1. `write_yolo_labels`: keep the `create_dir_all(parent)` logic, replace `fs::write` with
   `write_atomic(file_path, serialize_yolo_labels(labels).as_bytes())`.
2. `ClassIndex::save_to_disk`: same swap. This requires `yoda-data` to see the helper —
   it already depends on `yoda-core`.

## Testing

- Unit: write → read back equals input; write over an existing file replaces content;
  parent-dir creation still works (`write_creates_parent_dirs` test already exists —
  it should pass unchanged).
- Failure-injection is not practical portably; instead assert no `*.tmp*` residue remains
  in the directory after a successful write.

## Risks

- None functional. Slight write amplification (fsync per save) — imperceptible next to the
  class-index rewrite issue tracked in performance/04.
