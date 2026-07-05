# Use Case 08 — Configure Class Names and Colors for a Dataset

> **Goal:** make a dataset readable in YoDa — human class names instead of bare IDs, and a
> stable, meaningful color per class.
> **Actor:** whoever sets up the dataset for the team.
> **Mode:** configuration (files + environment), no UI interaction.

## Class names

Point `YODA_CLASS_INFO_YAML` at the dataset's Ultralytics YAML — usually the same file you
train with; YoDa reads only its `names:` key and ignores `path`/`train`/`val`/etc.:

```yaml
names:
  0: back_bumper
  1: front_bumper
  2: wheel
# or the list form:
names: [back_bumper, front_bumper, wheel]
```

Effects across the UI: object rows and dropdowns, class legend, filter chips (the filter bar
only renders when a class map exists), and the on-image *Class Name* chips. Class IDs found
in labels but absent from the YAML still display everywhere as `class <id>` — that's your
cue the YAML is stale.

If the file is missing or unparseable, YoDa starts anyway with an empty class map (numeric
names, no filter bar); nothing fails hard.

## Class colors

Without configuration, every class gets a deterministic auto color (golden-angle HSV): the
same ID always maps to the same hue, and consecutive IDs are visually distinct. Configure
`YODA_COLOR_MAP_YAML` only when you need specific colors (brand palettes, red-for-defect
conventions, matching another tool):

```yaml
0: "#e25d3f"          # hex string form
1: [127, 159, 75]     # RGB list form — both forms can be mixed
7: "#00c8ff"          # sparse: unlisted classes keep their auto color
```

An example lives at [`example/color_map.yaml`](../../example/color_map.yaml).

## Wiring it up

```bash
export YODA_CLASS_INFO_YAML="/data/carparts-seg.yaml"
export YODA_COLOR_MAP_YAML="/data/colors.yaml"
```

Both variables accept the shorter aliases `YODA_CLASS_INFO` / `YODA_COLOR_MAP`. A `.env`
file in the working directory works for local runs; in Docker pass `-e` flags and mount the
YAML files into the container.

**Changes require a restart** — both YAMLs are read at startup (the class map is technically
re-read per request server-side, but the UI fetches it once at load; treat both as
restart-to-apply).

## Verifying

1. Start YoDa and open any labeled image.
2. Legend and object rows show names, not `class <id>` → names YAML found and parsed.
3. Filter bar is present → same.
4. Swatches match your color map for overridden IDs → color YAML found.
5. If names are missing: check the startup log's `class_info` line (web server logs the
   resolved path, `<none>` if unset), and validate the YAML has a top-level `names:` key.

## Current limitations

- No hot-reload of either YAML and no UI for editing class metadata — file + restart is the
  workflow.
- Classes only in label files don't get filter chips
  ([fix designed](../implementation/correctness/09-filter-unknown-classes.md)).
- No dataset-wide class rename/merge tooling — editing the YAML renames the *display* only;
  renumbering label files is manual
  ([designed](../implementation/features/21-class-operations.md)).

## Related use cases

- [01 — Browse and inspect](01-browse-and-inspect-a-dataset.md)
- [09 — Serve a remote dataset](09-serve-a-remote-dataset.md)
