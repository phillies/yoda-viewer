# Honest Save Feedback (Saving… / Saved / Failed + Retry) and State Re-Sync

> Source: optimizations-and-features.md §5.2 · Effort: S–M
> Depends on: correctness/01 (warnings channel shares the same UI slot)

## Problem

`run_effects` (`crates/yoda-ui/src/lib.rs:1523-1545`) fires `save_labels` and forgets:

- No in-flight indication; rapid edits race (two PUTs can land out of order — last-writer-wins
  on the server, but the *client's* "Labels saved" message may describe the loser).
- On failure, `current_labels` silently diverges from disk with only a generic error string.
- The server's `LabelsResponse` from `PUT /api/labels` (fresh parse from disk) is discarded —
  the one authoritative view of what actually persisted.

## Design

### 1. Save-state machine in `AppState`

```rust
pub enum SaveState { Idle, Saving, Saved,            // Saved shows briefly then → Idle
                     Failed { message: String } }
pub save_state: SaveState,        // on AppState
pub save_generation: u64,         // bump per initiated save
```

Reducer actions: `SaveStarted(u64)`, `SaveSucceeded { generation: u64, labels: Vec<LabelObject> }`,
`SaveFailed { generation: u64, message: String }`. Stale-generation results (a newer save has
started) are ignored — this fixes the out-of-order race without request cancellation.

### 2. `run_effects` rework

```rust
AppEffect::PersistLabels { image_path, labels } => {
    let generation = next_generation(app_state);         // dispatch SaveStarted
    spawn(async move {
        match save_labels(...).await {                   // change to return LabelsResponse
            Ok(resp) => dispatch SaveSucceeded { generation, labels: resp.labels },
            Err(msg) => dispatch SaveFailed { generation, message: msg },
        }
    });
}
```

`SaveSucceeded` **replaces `current_labels` with the server's parsed truth** (only when the
image path still matches `current_image_path` — the user may have navigated away; compare and
drop otherwise). This closes the divergence loophole and makes server-side normalization
(coordinate formatting, index reassignment) visible immediately.

### 3. UI

- Status pill in the toolbar bound to `save_state`: `Saving…` (spinner-ish), `Saved ✓`
  (auto-clears — clear on next action rather than a timer, avoids timer plumbing), and
  `Save failed ↻` as a **button** that re-dispatches the persist effect with current labels.
- While `Failed`, add a subtle warning border to the object panel — the on-screen labels are
  not on disk.
- Navigation guard: attempting to switch images while `Failed` shows a confirm-style message
  ("unsaved changes will be lost — click again to discard"); while `Saving`, allow navigation
  (the PUT completes regardless; generation check discards the stale response).

### 4. Interplay with correctness/01

`LabelsResponse.warnings` (skipped lines) from the save response routes into the same status
area; a save that succeeded-with-warnings shows `Saved with N warnings`.

## Testing

- Reducer unit tests: generation staleness (older `SaveSucceeded` after newer `SaveStarted`
  is dropped), success replaces labels only when image matches, failure preserves labels and
  sets `Failed`.
- E2E: kill write permission on the temp label dir mid-test → change a class → `Save failed`
  appears; restore permission → retry button → `Saved`.

## Risks

- Replacing `current_labels` on success can clobber an edit made *during* the in-flight save.
  The generation guard prevents the stale case; the still-current case is correct by
  definition (the response reflects that same save). Edits made after the save but before its
  response bump the generation via their own save → response discarded. Sound.
