# Manual Mode

In **Auto** mode (default) the solver controls every slot on a page. **Manual** mode freezes the slot layout so the solver never touches it — useful for a specific page that needs a custom arrangement.

---

## Switching a page to manual mode

```bash
fotobuch page mode 3 manual     # page 3 → Manual
fotobuch page mode 3 auto       # page 3 → Auto (solver takes over again)
fotobuch page mode 3,5,7 manual # several pages at once
fotobuch page mode 2..6 manual  # page range
```

Shorthand: `m` = manual, `a` = auto.

---

## Repositioning slots (`page pos`)

The page must already be in Manual mode. All values are in millimetres relative to the page content area (excluding bleed):

```bash
# Relative move: right +20 mm, down +10 mm
fotobuch page pos 3:2 --by 20,10

# Absolute position: top-left corner at (50, 30) mm
fotobuch page pos 3:1 --at 50,30

# Scale to 1.5× (grows right and downward from current position)
fotobuch page pos 3:0 --scale 1.5

# Combine absolute position and scale
fotobuch page pos 3:0 --at 10,10 --scale 2.0
```

`--by` and `--at` are mutually exclusive. `--scale` can be combined with either.

---

## Notes

- `rebuild --page N` on a Manual page is a no-op — the solver is bypassed. Switch to Auto first if you want a fresh solver run.
- The page mode is stored in the YAML. Auto is the implicit default (the field is omitted for Auto pages).
