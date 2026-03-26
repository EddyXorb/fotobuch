# Customizing the Template

Every project has a `{name}.typ` file — a [Typst](https://typst.app/) template
that controls how the PDF looks. fotobuch generates this file for you, but you
are free (and encouraged) to edit it.

The top of the template has a clearly marked **USER SETTINGS** section. These
are the knobs you can turn without understanding the rest of the template.

> **Important:** Always edit `{name}.typ`, never `final.typ`. The final
> template is auto-generated from yours during `build release` (with
> `is_final = true`). Your changes in `final.typ` would be overwritten.

---

## Preview overlays

These settings only affect the preview PDF. They are automatically disabled in
the release build.

| Setting | Default | Effect |
|---|---|---|
| `show_image_captions_on_preview` | `false` | Show the filename on each photo |
| `show_borders_on_preview` | `true` | Red bleed border + blue margin border |
| `show_slot_info_on_preview` | `true` | Slot address and weight on each photo (e.g. `3:2 (1.5)`) |

```typ
#let show_image_captions_on_preview = false
#let show_borders_on_preview = true
#let show_slot_info_on_preview = true
```

Turn on `show_image_captions_on_preview` when you're trying to identify which
photo is where. Turn off `show_slot_info_on_preview` once you're happy with the
layout and just want a clean preview.

---

## Photo index (appendix)

The template can append a photo index at the end of the book — a compact
reference listing every photo with its group, timestamp, and a reference back to
its page position.

| Setting | Default | Effect |
|---|---|---|
| `appendix_show` | `false` | Enable the appendix |
| `appendix_nr_columns` | `7` | Number of columns in the listing |
| `appendix_ref_mode` | `"positions"` | How photos are referenced (see below) |
| `appendix_show_page_nr_separator` | `false` | Show a page-number header between pages |
| `appendix_try_strip_datetimes_from_photo_name` | `true` | Strip leading timestamps from filenames |
| `appendix_label_title` | `"Bildverzeichnis"` | Title text (change for your language) |
| `appendix_label_page` | `"Seite"` | "Page" label (change for your language) |

### Reference modes

**`"positions"`** (default) — Each photo is referenced as `page.slot`, e.g.
`2.3` means page 2, slot 3. No visual badge is added to the photos.

**`"counter"`** — Photos are numbered sequentially (1, 2, 3, …) and a small
badge with the number appears in the bottom-right corner of each photo in the
PDF.

```typ
#let appendix_show = true
#let appendix_ref_mode = "counter"
#let appendix_label_title = "Photo Index"
#let appendix_label_page = "Page"
```

---

## Going further

Everything below the `USER SETTINGS` block is the full Typst template: page
setup, slot rendering, cover logic, watermarks, and the appendix renderer. If
you know Typst, you can customize anything — fonts, colours, page numbering,
decorative elements, or the layout structure itself.

A few things to keep in mind:

- The template reads layout data from `{name}.yaml` via `#let data = yaml(…)`.
  The YAML structure is stable, so your customizations won't break on updates.
- `is_final` controls preview vs. release mode. Use it to conditionally show or
  hide elements: `#if not is_final [Draft watermark]`.
- Image paths are resolved relative to the project root via `cache_prefix`.
  Preview images live in `.fotobuch/cache/{name}/preview/`, final images in
  `.fotobuch/cache/{name}/final/`.

If you want to start over with the default template, create a fresh project with
`fotobuch project new` and copy the generated `{name}.typ` back.
