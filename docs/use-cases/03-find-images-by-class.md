# Use Case 03 — Find Images Containing Specific Classes

> **Goal:** narrow a large dataset down to the images that contain particular classes —
> "show me every image with a `person`", or "every image with both `car` and `bicycle`".
> **Actor:** anyone reviewing, debugging, or curating per class.
> **Mode:** read-only.

## Preconditions

- A dataset YAML (`YODA_CLASS_INFO_YAML`) is configured — the filter bar only appears when a
  class map exists.
- First server start on a large dataset: the class index is built by scanning every label
  file once (tens of seconds on big datasets; subsequent starts load the cache instantly).
  The scan happens during startup, before the server accepts connections.

## Walkthrough

1. **Open the filter bar** at the top of the left panel: one chip per class, in class colors.
2. **Select classes.** Click a chip to activate it (outlined in the class color). Click again
   to deselect. Select as many as needed.
3. **Choose the combination mode:**
   - **Any** (default) — images containing *at least one* selected class;
   - **All** — images containing *every* selected class (e.g. `car` + `bicycle` finds
     co-occurrences).
4. **Read the filtered tree.** Only matching images remain; folders that contain no matches
   disappear entirely. Folder expand/collapse still works. The status bar shows an active
   filter badge listing the selected class names.
5. **Work the result set.** Click through the remaining images — combined with the class
   legend's *Hide* buttons you can inspect exactly the class you filtered for
   (use case [04](04-focus-in-crowded-scenes.md)).
6. **Clear the filter** with `× Clear` in the filter bar, or the `×` on the status-bar badge.

## How matching works

The filter matches on **class presence in the label file** (from the persistent class
index), not on what is currently visible. Hidden classes/objects still count. Images whose
label files were edited *outside* YoDa while the server is running may match stale data —
restart the server to refresh the index (see limitations).

## Expected results

- Filtering is instant even on large datasets (the index is fully client-side after load).
- Edits made through YoDa (class changes, deletes, new polygons) update the index
  immediately — the filtered tree reflects your own edits without a restart.

## Current limitations

- Chips only exist for classes in the dataset YAML. Class IDs that occur *only* in label
  files (e.g. a stray `class 17`) cannot be filtered for
  ([fix designed](../implementation/correctness/09-filter-unknown-classes.md)).
- No "unlabeled images" filter
  ([designed](../implementation/features/08-unlabeled-surfacing.md)).
- No filename search to combine with the class filter
  ([designed](../implementation/features/06-filename-search.md)).
- External label edits are invisible until restart — there is no rebuild button or file
  watcher yet ([designed](../implementation/features/10-class-index-refresh.md)).
- The first-run index build shows no progress indication
  ([designed](../implementation/features/07-index-build-progress.md)).

## Related use cases

- [01 — Browse and inspect](01-browse-and-inspect-a-dataset.md)
- [02 — Review annotation quality](02-review-annotation-quality.md)
- [04 — Focus in crowded scenes](04-focus-in-crowded-scenes.md)
