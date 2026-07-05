# Use Case 02 — Review Annotation Quality

> **Goal:** systematically judge whether masks/boxes are accurate, complete, and correctly
> classed — e.g. QA before a training run, or auditing labels delivered by an annotation
> vendor.
> **Actor:** ML engineer, annotation lead.
> **Mode:** read-only for pure review; unlocked if fixing while reviewing (see use cases
> [05](05-correct-object-classes.md)–[07](07-add-missing-annotations.md)).

## Preconditions

As in [use case 01](01-browse-and-inspect-a-dataset.md); class names YAML strongly
recommended — most QA findings are "right shape, wrong class", which you can only spot with
names on.

## Walkthrough

1. **Set up the view for the check you're doing.**
   - *Mask accuracy:* **Mask** on, **BBox** off, **Class Name** on.
   - *Box tightness:* **BBox** on, **Mask** off.
   - *Mask ↔ box agreement:* both on — polygon labels show their derived box dashed; a big
     gap between dashed box and mask means stray polygon points.
2. **Zoom into boundaries.** Wheel-zoom to object edges. Typical defects that become obvious
   at 3–6×: masks cutting into the object, background bleed, blocky low-vertex polygons,
   duplicated overlapping annotations.
3. **Use the object list as a checklist.** For each image, read the right panel top to
   bottom: does the count match what you see? Does each `#n class (type)` entry correspond
   to a real object?
4. **Isolate objects in crowded scenes.** Hide everything else:
   - class-level: *Hide* button per class in the *Classes* panel;
   - object-level: the *Visible* button per row in the *Objects* panel.
   With only one object visible, boundary quality is much easier to judge. Selecting a row
   also highlights its shape (white dashed outline) without hiding anything.
5. **Check per-class systematically.** Use the class filter (use case
   [03](03-find-images-by-class.md)) to walk through all images of one class at a time —
   reviewing "all `wheel` images" back-to-back surfaces systematic labeling drift far better
   than random sampling.
6. **Record findings.** YoDa has no built-in review states yet — track findings externally
   (spreadsheet of image paths). Copy the image name from the status bar.

## Expected results

- Every visible defect maps to an identifiable object row (index, class, type), so findings
  are precisely addressable as *image path + object #n*.
- Hiding classes/objects never modifies data — it is pure view state and resets on reload.

## Current limitations

- No review status / notes / audit trail
  ([designed](../implementation/bold/04-multi-user-review.md)).
- No dataset-level statistics to target the review (class imbalance, objects-per-image)
  ([designed](../implementation/features/20-stats-dashboard.md)).
- No unlabeled-image surfacing — images with zero annotations must be spotted manually
  ([designed](../implementation/features/08-unlabeled-surfacing.md)).
- No comparison against model predictions
  ([designed](../implementation/bold/03-prediction-review-mode.md)).
- Selection highlight is a static dashed outline (the marching-ants animation is currently
  inert — [fix designed](../implementation/correctness/04-selection-animation.md)).

## Related use cases

- [04 — Focus in crowded scenes](04-focus-in-crowded-scenes.md)
- [05 — Correct object classes](05-correct-object-classes.md) /
  [06 — Delete bad annotations](06-delete-bad-annotations.md) /
  [07 — Add missing annotations](07-add-missing-annotations.md)
