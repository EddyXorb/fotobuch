# Cover Workflow Improvement

## Problem

After the first `build`, the cover (page 0) is treated like a regular page: the
solver scatters multiple photos across it. The user must manually clear the cover,
swap in the desired photo, and rebuild. There is no way to express common cover
layouts like "one photo on the front only" or "one photo spanning the whole spread".

## Solution: `mode` field in cover config

A new `mode` field in `config.book.cover` defines a fixed slot layout for the
cover page. When a mode other than `free` is set, the GA-solver is bypassed and
slot positions are calculated deterministically.

### Available modes

| Mode | Slots | Behaviour |
|------|-------|-----------|
| `free` | any | GA-solver optimises freely (current behaviour) |
| `front` | 1 | Photo on front cover (right half), aspect ratio preserved, centred |
| `front-full` | 1 | Photo fills entire front cover (may crop) |
| `back` | 1 | Photo on back cover (left half), aspect ratio preserved, centred |
| `back-full` | 1 | Photo fills entire back cover |
| `spread` | 1 | Photo spans full spread (over spine), aspect ratio preserved, centred |
| `spread-full` | 1 | Photo fills full spread |
| `split` | 2 | Slot 0 = front, Slot 1 = back, aspect ratio preserved, centred |
| `split-full` | 2 | Slot 0 = front, Slot 1 = back, each half fully filled |

**Default:** `free` (no behaviour change for existing projects).

### Config

```yaml
config:
  book:
    cover:
      active: true
      mode: front                # new field
      spine_clearance_mm: 5.0    # new: gap between photo edge and spine (split/front/back)
      # existing fields unchanged:
      front_back_width_mm: 594.0
      height_mm: 297.0
      spine_mode: auto
      spine_mm_per_10_pages: 1.4
      spine_text: "Italy 2024"
```

### Layout calculation

For modes other than `free`, slot positions are computed deterministically:

- **Half-page width** = `(front_back_width_mm - spine_width) / 2`
  (spine_width from auto or fixed config)
- **Front area** = right half of the spread (x from midpoint + spine/2 + clearance)
- **Back area** = left half of the spread (x from 0 to midpoint - spine/2 - clearance)
- **Aspect-ratio modes** (`front`, `back`, `spread`, `split`): photo is scaled to
  fit maximally within its target area while preserving its aspect ratio, then centred.
- **Full modes** (`front-full`, `back-full`, `spread-full`, `split-full`): slot is
  set to exactly match the target area dimensions. The photo fills the slot completely
  (the template handles cropping/scaling).
- **`spread` / `spread-full`**: the spine is ignored — the photo goes across it.

### Workflow examples

**Single photo on front cover:**
```bash
fotobuch place cover.jpg --into 0
# config has: cover.mode: front
fotobuch rebuild --page 0
```

**Panorama across full spread:**
```bash
fotobuch place panorama.jpg --into 0
# config has: cover.mode: spread-full
fotobuch rebuild --page 0
```

**Two photos, front and back:**
```bash
fotobuch place front.jpg --into 0
fotobuch place back.jpg --into 0
# config has: cover.mode: split
# Slot 0 (first placed) → front, Slot 1 → back
fotobuch rebuild --page 0
```

**Let the solver decide (existing behaviour):**
```bash
# config has: cover.mode: free
fotobuch build   # solver treats cover like any page
```

## Implementation steps

- [ ] Add `mode: CoverMode` enum to `CoverConfig` (`free`, `front`, `front-full`,
      `back`, `back-full`, `spread`, `spread-full`, `split`, `split-full`)
- [ ] Add `spine_clearance_mm: f64` field to `CoverConfig` (default 5.0)
- [ ] Implement deterministic slot calculation in `rebuild_single_page.rs`:
      when page 0 and mode != `free`, compute slot positions from cover dimensions
      instead of running the GA-solver
- [ ] Validate slot count matches mode (e.g. `split` requires exactly 2 photos
      on page 0, `front` requires exactly 1) — emit clear error otherwise
- [ ] Update `fotobuch page info 0` to show the active cover mode
- [ ] Update `configuration.md` with new cover fields
- [ ] Update `known_limitations.md` (cover is no longer "rough around the edges"
      when a mode is set)

## Future generalisation

See `TODO.md` — the mode concept can be extended to inner pages with `left`/`right`
naming and an additional gutter mode for non-lay-flat bindings.
