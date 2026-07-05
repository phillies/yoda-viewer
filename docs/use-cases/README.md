# YoDa Viewer — Use Cases (current version)

> Task-oriented walkthroughs of everything the **current** version supports. Each document
> covers one job: goal, preconditions, step-by-step walkthrough, behavior details, and the
> current limitations with links to the corresponding design docs in
> [`../implementation/`](../implementation/README.md). Feature-by-feature reference lives in
> the [README](../../README.md#feature-documentation).

## The use cases

| # | Use case | One-liner | Mode |
|---|----------|-----------|------|
| [01](01-browse-and-inspect-a-dataset.md) | Browse & inspect a dataset | First look: structure, images, overlays, zoom | read-only |
| [02](02-review-annotation-quality.md) | Review annotation quality | Systematic QA of masks/boxes/classes before training | read-only |
| [03](03-find-images-by-class.md) | Find images by class | Filter the tree to images containing selected classes (Any/All) | read-only |
| [04](04-focus-in-crowded-scenes.md) | Focus in crowded scenes | Hide classes/objects, select & zoom to judge one annotation | read-only |
| [05](05-correct-object-classes.md) | Correct object classes | Fix "right shape, wrong class" via the dropdown; auto-saved | editing |
| [06](06-delete-bad-annotations.md) | Delete bad annotations | Remove duplicates/spurious objects; auto-saved | editing |
| [07](07-add-missing-annotations.md) | Add missing annotations | Draw a new segmentation polygon; auto-saved | editing |
| [08](08-configure-dataset-metadata.md) | Configure dataset metadata | Class names + colors via YAML | setup |
| [09](09-serve-a-remote-dataset.md) | Serve a remote dataset | Docker / bare binary / SSH tunnel; security posture | deployment |
| [10](10-desktop-local-review.md) | Desktop local review | Native window with a self-hosted backend | deployment |
| [11](11-scripting-with-the-api.md) | Script with the API | Headless checks, extraction, and bulk edits over HTTP | automation |

## How they compose

A typical dataset-QA session chains them:

```
08 configure ─▶ 09/10 deploy ─▶ 01 orient ─▶ 03 filter to a class
                                     │
                                     ▼
                        02 review  ⇄  04 declutter
                                     │ findings
                                     ▼
                        05 reclass / 06 delete / 07 draw   (unlock once)
                                     │
                                     ▼
                        11 script: verify with the API / export lists
```

## Capability boundaries of the current version (read once)

The honest summary of where the current version stops — each links to its design doc:

- **Editing is per-object, immediate, and irreversible** — no undo
  ([19](../implementation/features/19-undo-redo.md)), no multi-select
  ([13](../implementation/features/13-multi-select-batch-ops.md)), no vertex editing
  ([16](../implementation/features/16-vertex-editing.md)), no bbox drawing
  ([05](../implementation/features/05-bbox-draw-mode.md)).
- **Navigation is click-driven** — no prev/next or arrow keys
  ([01](../implementation/features/01-prev-next-navigation.md)), only three keyboard
  shortcuts ([02](../implementation/features/02-keyboard-shortcuts.md)), no filename search
  ([06](../implementation/features/06-filename-search.md)).
- **Session state is ephemeral** — last image, toggles, visibility, and filter reset on
  reload ([04](../implementation/features/04-last-image-persistence.md),
  [14](../implementation/features/14-visibility-persistence.md)).
- **Single-user assumptions** — no auth or read-only switch
  ([12](../implementation/features/12-read-only-mode.md)), last-writer-wins on concurrent
  edits ([bold/04](../implementation/bold/04-multi-user-review.md)), external label edits
  invisible until restart ([10](../implementation/features/10-class-index-refresh.md)).
- **One data-safety trap**: label files with malformed lines load as *empty*, and a
  subsequent edit rewrites the file without the unparsed lines — don't edit an image that
  unexpectedly shows 0 objects
  ([fix designed, top priority](../implementation/correctness/01-partial-label-parse.md)).
