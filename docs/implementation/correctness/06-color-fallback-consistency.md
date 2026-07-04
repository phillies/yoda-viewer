# Consistent Color Fallback for Class IDs ≥ 100

> Source: optimizations-and-features.md §2.6 · Effort: XS
> Depends on: nothing

## Problem

`load_color_map` (`crates/yoda-config/src/lib.rs:174-207`) pre-fills defaults for class IDs
0–99 only. `YoDaConfig::get_color_tuple` / `get_color_string` (lib.rs:138-151) fall back to
**white** for anything not in the map, while every other call site
(`yoda-ui/src/lib.rs`, `yoda-web/src/lib.rs`, `render.rs:92-94`) falls back to
`default_color_for_class(id)` — the golden-angle HSV color. A dataset with 120 classes gets
correct colors in the overlay but white in anything using the config accessors, and the 0–100
pre-fill loop wastes 100 map entries per load for no benefit.

## Design

1. Delete the `for class_id in 0..100` pre-fill loop in `load_color_map`; the returned maps
   then contain **only** user-provided entries.
2. Change the accessors to compute the default:

```rust
pub fn get_color_tuple(&self, class_id: u32) -> (u8, u8, u8) {
    self.color_map_tuples.get(&class_id).copied()
        .unwrap_or_else(|| default_color_for_class(class_id))
}
```

   (same for `get_color_string`, formatting the tuple).
3. Audit consumers of the *map itself*: `yoda-data::color_map()` and the `/api/color-map`
   endpoint will now return a smaller map. The UI already applies
   `unwrap_or_else(default_color_for_class)` everywhere it reads `color_map()`
   (e.g. `yoda-ui/src/lib.rs:1018, 1038, 1132`), so behavior converges rather than breaks —
   and the WASM `default_color_for_class` is identical code.

## Testing

Update existing `yoda-config` tests:
- `default_color_map_has_100_entries` → replaced by `empty_when_no_yaml` (asserts empty maps).
- `get_color_string_fallback` / `get_color_tuple_fallback` → assert
  `default_color_for_class(9999)` instead of white.
- `custom_color_map_overrides` → drop the `contains_key(&50)` assertion.
- New: IDs above 100 with a custom YAML entry still override the generated default.

## Risks

Anyone relying on white-for-unknown as a visual "missing color" cue loses that signal — but
the rest of the app never had it, so this removes an inconsistency rather than a feature.
