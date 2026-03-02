# YoDa – UX Improvement Concept (V4)

## Current Problems

Based on a visual audit of the running application:

### 1. Readability & Contrast
- **Toolbar text is nearly invisible**: "Display:", "Seg. Masks", "Bounding Boxes", "Class ID", "Class Name" labels are dark/muted against the `bg-gray-800` toolbar — hard to read
- **File tree text is tiny** and uses default NiceGUI blue links on white — poor hierarchy
- **Object list text** in the right drawer is very small (`text-xs`) white on dark — functional but strained
- **Class legend checkboxes** are tiny (`size=xs`) and hard to click
- **Switch labels** disappear into the dark toolbar; the switch tracks (green/blue/orange/purple) don't help the label text

### 2. Chunky / Cluttered Layout
- **Toolbar is a single cramped row** with everything jammed together: display toggles, mode buttons, class selector, zoom controls, drawer toggle — wraps awkwardly on smaller screens
- **File tree panel is too wide** (20% splitter default) for what it shows, and filenames are absurdly long hashes that overflow
- **Right drawer is 300px wide** and feels heavy; object rows are packed with too many inline controls (eye, dot, index, dropdown, type badge, delete) all on one line
- **No visual grouping** — toolbar items have thin vertical separators but no clear sections
- **No breathing room** — minimal padding/margins throughout

### 3. Visual Design
- **No consistent color palette** — random mix of gray-800, gray-900, white, yellow, red, green, blue, orange, purple
- **Buttons have no visual affordance** — flat icon buttons blend into the toolbar
- **No hover/focus states** — interactive elements don't respond visually
- **Active mode button** uses yellow highlight which is garish and low-contrast
- **Selected object** in drawer uses a barely-visible `rgba(255,255,255,0.12)` background
- **The "100%" text button** looks out of place among icon buttons

### 4. Functional UX Issues
- **No status bar** — no feedback about current image path, dimensions, label count
- **No keyboard shortcut hints** in tooltips
- **Class selector in toolbar** is borderless and blends in — unclear it's interactive
- **Delete button (trash)** in toolbar is always visible but greyed out when nothing is selected — confusing

---

## Proposed Design: Clean Dark Theme

### Color System

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-base` | `#1e1e2e` (dark navy) | Main canvas background |
| `--bg-surface` | `#282840` | Panels, drawers, toolbar |
| `--bg-surface-hover` | `#32325a` | Hover states on surface |
| `--bg-elevated` | `#363660` | Cards, dropdowns, selected rows |
| `--text-primary` | `#e0e0f0` | Main text — high contrast on dark |
| `--text-secondary` | `#a0a0c0` | Secondary labels, hints |
| `--text-muted` | `#6a6a90` | Disabled, placeholder text |
| `--accent` | `#7c8aff` | Primary actions, active state |
| `--accent-hover` | `#9aa4ff` | Hover on accent |
| `--danger` | `#ff6b6b` | Delete buttons |
| `--success` | `#69db7c` | Positive actions, active toggles |
| `--border` | `#3a3a60` | Subtle borders/separators |

### Layout Redesign

```
┌─────────────────────────────────────────────────────────────────────┐
│ TOOLBAR (compact, 40px height)                                      │
│ ┌─────────┐ ┌───────────────────────┐ ┌──────────┐ ┌──────┐ ┌───┐ │
│ │ Display │ │ 🎭 Seg  📦 Box  🏷 ID│ │ ✋ ✏️ 🗑│ │ Zoom │ │ ☰ │ │
│ └─────────┘ └───────────────────────┘ └──────────┘ └──────┘ └───┘ │
├────────┬──────────────────────────────────────┬─────────────────────┤
│ FILES  │                                      │ INSPECTOR           │
│ 180px  │         IMAGE CANVAS                 │ 280px               │
│        │                                      │                     │
│ 📁test │                                      │ ── Classes ──────── │
│  img1  │   (image + SVG overlay)              │ ☑ back_bumper  ●    │
│  img2  │                                      │ ☑ back_door    ●    │
│  img3  │                                      │ ☑ back_glass   ●    │
│ 📁train│                                      │                     │
│ 📁val  │                                      │ ── Objects ──────── │
│        │                                      │ 👁 #1 back_light ▼ │
│        │                                      │ 👁 #2 trunk      ▼ │
│        │                                      │ 👁 #3 bumper     ▼ │
├────────┴──────────────────────────────────────┴─────────────────────┤
│ STATUS BAR (24px): car87.jpg | 1024×768 | 6 objects | Edit mode     │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Detailed Changes

### A. Toolbar — Compact & Grouped

**Current**: Single row, everything mixed, wraps badly.
**New**: Compact 40px bar with clearly grouped button clusters separated by styled dividers.

1. **Display toggles** — Replace verbose `ui.switch` with compact **icon toggle buttons** in a button group:
   - `layers` icon = Seg. Masks (tooltip: "Segmentation Masks")
   - `check_box_outline_blank` icon = Bounding Boxes
   - `tag` icon = Class ID
   - `label` icon = Class Name
   - Active toggles use `--accent` color fill; inactive are `--text-muted`
   - Much more compact — saves ~200px of toolbar width

2. **Mode cluster** — Edit / Draw / Delete as a segmented button group:
   - Edit (pan_tool) | Draw (edit) | Delete (delete_outline)
   - Active button has `--accent` background
   - Delete only shown/enabled when an object is selected
   - Class selector appears inline next to draw button only when in draw mode

3. **Zoom cluster** — Fit / Zoom In / Zoom Out / 100% as icon buttons in a group
   - Add current zoom level as a small label (e.g., "150%")

4. **Drawer toggle** — Single icon button at the far right

**Styling**:
- Background: `--bg-surface`
- Height: 40px fixed
- Button groups have `--border` colored rounded containers
- 8px gap between groups

### B. File Tree Panel — Slim & Clean

**Current**: 20% splitter, full filenames, default tree styling.
**New**:

1. **Reduce default width** to 15% (splitter value)
2. **Truncate filenames** with CSS `text-overflow: ellipsis` — show full name on hover tooltip
3. **Style the tree**:
   - Font: 13px, `--text-primary` color
   - Selected file: `--accent` left border + `--bg-elevated` background
   - Folder icons: use folder/folder_open Material icons
   - Remove default NiceGUI blue link color — use `--text-primary`
4. **Add subtle header** with folder path breadcrumb
5. **Panel background**: `--bg-surface` with right border `--border`

### C. Right Panel — Inspector (Always Visible)

**Current**: Right drawer that toggles open/closed, 300px, cramped rows.
**New**: Replace `ui.right_drawer` with a **fixed right panel** in the splitter layout (3-way split: tree | canvas | inspector), toggleable via the toolbar button.

1. **Width**: 280px default
2. **Class Legend section**:
   - Section header: "CLASSES" in `--text-secondary`, uppercase, 11px, letter-spaced
   - Larger checkboxes (size=sm instead of xs)
   - Color swatch: 14px × 14px rounded square (not circle — squares read better at small sizes)
   - Class name: 13px, `--text-primary`
   - Row height: 28px with hover highlight (`--bg-surface-hover`)

3. **Objects section**:
   - Section header: "OBJECTS" + object count badge
   - Each object in a **card-like row** (48px height, 4px border-radius, `--bg-surface` bg):
     ```
     ┌────────────────────────────────────┐
     │ 👁  ● #1  [back_light     ▼]  🗑  │
     │     color  idx  class dropdown del │
     └────────────────────────────────────┘
     ```
   - **Selected object**: `--accent` left border (3px) + `--bg-elevated` background
   - **Eye icon**: larger touch target (24px), toggles between `visibility`/`visibility_off`
   - **Color dot**: 10px, circle, matches class color
   - **Index**: bold, `--text-secondary`
   - **Class dropdown**: styled with `--bg-elevated` background, visible border
   - **Delete button**: only visible on row hover (reduces visual clutter)
   - **Type badge [poly/bbox]**: remove entirely — low value, wastes space

### D. Image Canvas

**Current**: `bg-gray-900` background.
**New**:
- Background: `--bg-base` with a subtle checkerboard pattern (CSS only, like Photoshop transparency indicator) to help distinguish image edges
- Keep all existing zoom/pan functionality unchanged

### E. Status Bar (New)

Add a status bar at the bottom (24px height):
- **Left**: Current file path (truncated with ellipsis)
- **Center**: Image dimensions (e.g., "1024 × 768")
- **Right**: Object count + current mode indicator
- Background: `--bg-surface`, text: `--text-secondary`, font: 12px monospace

### F. SVG Overlay Improvements

1. **Text labels**: Increase font-size from 12 → 14px, use sans-serif instead of monospace for better readability
2. **Text background**: Use rounded rect with more padding (6px horizontal, 3px vertical) instead of tight box
3. **Selected polygon**: Use a brighter highlight — white stroke + animated dash (`stroke-dashoffset` CSS animation) for a "marching ants" effect
4. **Non-selected polygon**: Reduce fill-opacity from 0.4 → 0.3 for less visual noise, so the image is clearer

### G. Global CSS Improvements

```css
/* Inject via ui.add_head_html */

/* Smooth transitions on all interactive elements */
button, .q-toggle, .q-checkbox, .q-select {
    transition: background-color 0.15s ease, color 0.15s ease,
                border-color 0.15s ease, opacity 0.15s ease;
}

/* Custom scrollbar for dark theme */
::-webkit-scrollbar { width: 8px; }
::-webkit-scrollbar-track { background: var(--bg-base); }
::-webkit-scrollbar-thumb { background: var(--border); border-radius: 4px; }
::-webkit-scrollbar-thumb:hover { background: var(--text-muted); }

/* File tree: truncate long names */
.q-tree__node-header-content {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
}

/* Tooltip styling */
.q-tooltip {
    font-size: 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
}
```

### H. Keyboard Shortcut Hints

Add shortcut hints to tooltips:
- Edit mode: "Edit mode (E)"
- Draw mode: "Draw mode (D)"
- Delete: "Delete selected (Del)"
- Fit to screen: "Fit to screen (F)"
- Zoom in: "Zoom in (+)"
- Zoom out: "Zoom out (-)"
- Toggle masks: "Toggle masks (M)"
- Toggle bboxes: "Toggle bboxes (B)"

---

## Summary of Impact

| Area | Before | After |
|------|--------|-------|
| Toolbar height | ~44px, wraps | 40px fixed, never wraps |
| Toolbar controls | 11 individual elements | 4 grouped clusters |
| Text readability | Dark on dark, tiny | High-contrast, sized for legibility |
| File panel | 20%, overflowing names | 15%, truncated + tooltip |
| Right panel | Drawer, toggle to open | Fixed panel, always accessible |
| Object rows | 6+ inline controls | Clean card, hover-reveal delete |
| Status feedback | None | Persistent status bar |
| Color system | Ad-hoc | Consistent token-based palette |
| Transitions | None | 150ms ease on all interactives |
| Keyboard hints | None | Shortcuts in all tooltips |

---

## Implementation Notes

- All visual changes are in `ui.py` (and custom CSS via `ui.add_head_html`)
- SVG rendering changes are in `label.py` (`_render_segmask`, `_render_text_label`, `_render_bbox`)
- No changes to data model, config, or file operations
- Existing E2E tests may need selector updates if element structure changes
- CSS custom properties (variables) should be defined once in `<style>` block for consistency
