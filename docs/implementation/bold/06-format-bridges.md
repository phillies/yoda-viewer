# Format Bridges: COCO / Pascal VOC Import & Export

> Source: optimizations-and-features.md §7.6 · Effort: L
> Depends on: nothing hard; performance/03's `LabelWire`/hydrate machinery helps ·
> features/20's export endpoints set the download plumbing pattern

## Concept

Read/write COCO JSON and Pascal VOC XML alongside YOLO txt. Import makes YoDa the viewer of
record for datasets from anywhere; export makes it a converter people arrive for. The
`yoda-core` label model already carries everything COCO detection/segmentation and VOC
detection need.

## Design principles

- **Codec modules, not a new storage model.** YOLO txt stays the on-disk truth. Import =
  one-time conversion *into* YOLO layout; export = one-time generation *from* it. YoDa does
  not natively browse a COCO file (that's a different architecture; refuse the temptation).
- New crate `yoda-formats` (keeps serde-heavy schema types out of core):

```rust
pub trait DatasetImporter {
    fn inspect(&self, source: &Path) -> Result<ImportPlan>;   // dry-run: counts, classes, issues
    fn import(&self, source: &Path, dest: &ImportDest, progress: …) -> Result<ImportReport>;
}
// ImportDest { image_root, label_root, yaml_out }
```

## COCO codec

- Import: `instances_*.json` (detection + segmentation). Mapping notes:
  - category ids → contiguous YOLO ids (COCO ids are sparse); emit the mapping into the
    generated dataset YAML `names:`.
  - polygons: COCO segmentation lists (multi-polygon per annotation → one YOLO line per
    polygon, same class — document the object-count inflation); RLE masks → **skip with
    per-annotation warning in v1** (decode+contour later, shares bold/01's maskpoly).
  - bbox `[x,y,w,h]` absolute → normalized cx,cy,w,h.
  - images referenced by `file_name`: import expects the image files present (copy or
    reference-in-place — plan option `link_images: bool`, default reference: just write
    labels mirroring the existing image tree).
- Export: walk flat index + labels → single JSON; polygons → segmentation + computed bbox +
  area; `info/licenses` minimal. Streaming write (`serde_json::to_writer`) — COCO files get
  big.

## VOC codec

- Import: per-image XML (`<object><bndbox>`), name→id via dataset YAML or generated;
  no polygons in classic VOC — bbox only.
- Export: one XML per image (bbox objects only; polygons exported as their bounding boxes
  with a warning count in the report — or excluded, plan option; default exclude to avoid
  silent geometry loss).

## Surfaces

- **CLI-first** (natural fit, pairs with bold/08): `yoda convert --from coco.json --to-yolo
  out/ --images imgs/`, `yoda export --format coco --out annotations.json` with `--filter`
  reusing filter semantics. Conversion is scriptable batch work; a CLI is the honest UI.
- Web endpoints second: `GET /api/export/coco?filter=…` streaming download; import via UI is
  file-upload + server paths complexity — defer, CLI covers it.

## Testing

- Roundtrip properties: YOLO → COCO → YOLO preserves geometry within float tolerance
  (class mapping identity); same for VOC bbox.
- Golden fixtures: a hand-written 3-image COCO file (incl. one multi-polygon, one RLE to
  assert the skip-warning) and VOC set; snapshot the generated YOLO txt + YAML.
- Fuzz-ish: malformed JSON/XML → clean errors with file/annotation context, never panic
  (the `unwrap_used` lint already helps).

## Risks

- COCO dialects (keypoints, crowd flags, licenses omitted, tools' quirks) — scope statement
  in `inspect` output: unsupported fields are counted and named, not silently dropped.
- The importer writes many files — reuse features/21's backup/dry-run ethos: `inspect` is
  mandatory in the CLI flow (auto-run before `import`, `--yes` to skip prompt).
