# Config Enhancement: Move Template Settings to YAML

## Problem

Several settings that control the look of the generated PDF currently live as
`#let` variables in the Typst template (`{name}.typ`). This forces users to
edit a Typst source file — which is intimidating for non-programmers and
error-prone. These settings should be controlled via the YAML config, like
everything else.

## Current template settings

The following variables are defined in the "USER SETTINGS" section of the
template and are read by the Typst rendering logic:

### Preview overlays

| Template variable | Current default | Purpose |
|---|---|---|
| `show_image_captions_on_preview` | `false` | Show filename on each photo |
| `show_borders_on_preview` | `true` | Red bleed border + blue margin border |
| `show_slot_info_on_preview` | `true` | Slot address and weight on each photo |

### Appendix (photo index)

| Template variable | Current default | Purpose |
|---|---|---|
| `appendix_show` | `false` | Enable photo index at end of book |
| `appendix_nr_columns` | `7` | Number of columns in the listing |
| `appendix_ref_mode` | `"positions"` | Reference style: `"positions"` or `"counter"` |
| `appendix_show_page_nr_separator` | `false` | Page-number headers between pages |
| `appendix_try_strip_datetimes_from_photo_name` | `true` | Strip timestamps from filenames |
| `appendix_label_title` | `"Bildverzeichnis"` | Title of the appendix |
| `appendix_label_page` | `"Seite"` | "Page" label text |

### Build mode

| Template variable | Current default | Purpose |
|---|---|---|
| `is_final` | `false` | Preview vs. release mode (set automatically by `build release`) |

`is_final` is already managed by fotobuch (rewritten to `true` in `final.typ`).
It should stay implicit — not exposed in config.

---

## Proposed new config sections

### `config.preview` (extend existing)

```yaml
config:
  preview:
    # Existing fields
    show_filenames: false         # renamed from show_image_captions_on_preview
    max_preview_px: 800
    # New fields
    show_borders: true           # was: show_borders_on_preview (bleed + margin)
    show_slot_info: true         # was: show_slot_info_on_preview
```

All preview overlay settings go here because they only affect the preview PDF.

### `config.book.appendix` (new nested section)

```yaml
config:
  book:
    appendix:
      active: false              # was: appendix_show
      columns: 7                 # was: appendix_nr_columns
      ref_mode: "positions"      # was: appendix_ref_mode ("positions" | "counter")
      page_separator: false      # was: appendix_show_page_nr_separator
      strip_timestamps: true     # was: appendix_try_strip_datetimes_from_photo_name
      label_title: "Photo Index" # was: appendix_label_title (switch default to English)
      label_page: "Page"         # was: appendix_label_page (switch default to English)
      date_format: "{day}. {month} {year} {hour}:{min} Uhr"
      date_months: ["Jan", "Feb", "Mär", "Apr", "Mai", "Jun",
                    "Jul", "Aug", "Sep", "Okt", "Nov", "Dez"]
```

The appendix is part of the book output (it appears in both preview and
release), so it belongs under `config.book` rather than `config.preview`.

---

## Migration strategy

1. **Add the new config structs** (`AppendixConfig`, extend `PreviewConfig`)
   with serde defaults matching the current template defaults.

2. **Emit the values into the YAML data** that the Typst template reads
   (the template already reads `data.config` via `#let data = yaml(…)`).

3. **Update the template** to read these values from `data.config` instead of
   from `#let` declarations. The first lines of the template (the current
   "USER SETTINGS" block, lines 1–25) should be replaced with a short comment
   that tells users where these settings now live:
   ```typst
   // Settings like preview overlays and the appendix are now in your
   // YAML config file. Run `fotobuch config` to see all options.
   ```
   This keeps the template's first lines informative for anyone who opens it.

4. **Keep backward compatibility** for one release: if the template still has
   `#let` overrides and the YAML has defaults, the template-local value wins.
   This lets existing users migrate at their own pace.

5. **Remove the `#let` declarations** from the template in a later release
   once the migration period is over.

---

## Implementation steps

- [ ] Add `AppendixConfig` struct to `src/dto_models/config/`
- [ ] Add `show_borders` and `show_slot_info` fields to `PreviewConfig`
- [ ] Nest `AppendixConfig` under `BookConfig` as `appendix`
- [ ] Ensure `fotobuch config` prints the new fields
- [ ] Update template generator (`new/template.rs`) to emit config-aware
      template that reads from `data.config.preview` and `data.config.book.appendix`
- [ ] Update existing template: replace `#let` declarations with reads from
      YAML data, with fallback to old `#let` values for compatibility
- [ ] Update `template.md` documentation to reflect that settings are now in YAML
- [ ] Add new fields to `configuration.md`
- [ ] Default `appendix_label_title` and `appendix_label_page` to English
      (breaking change for existing German defaults — acceptable for v0.1.0)
- [ ] Add `date_format` and `date_months` fields to `AppendixConfig`
- [ ] Update template `fmt_ts_de()` to read format string and month names from
      `data.config.book.appendix` instead of hardcoded German values
