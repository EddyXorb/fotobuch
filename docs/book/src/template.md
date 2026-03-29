# Customizing the Template

Every project has a `{name}.typ` file — a [Typst](https://typst.app/) template
that controls how the PDF looks. ^*fotobuch* generates this file for you, but you
are free (and encouraged) to edit it, if you are proficient with typst.
Tweak it to your needs, use it as a starting point. fotobuch will not overwrite your template once you created it, so your changes stay safe within the project folder and can also be reused in other fotobuch projects.
Make sure that during your edits you do not change the lines

```typst
#let is_final = false
#let project_name = "{project_name}"
#let data = yaml(project_name + ".yaml")
#set text(font: "Libertinus Serif")
```

otherwise it won't compile.

> **Important:** Always edit `{name}.typ`, never `final.typ`. The final
> template is auto-generated from yours during `build release` (with
> `is_final = true`). Your changes in `final.typ` would be overwritten.

---

## Preview overlays

These settings only affect the preview PDF. They are automatically disabled in
the release build. Configure them in `{name}.yaml`:

```yaml
config:
  preview:
    show_filenames: false    # show filename caption on each photo
    show_borders: true       # red bleed border + blue margin border
    show_slot_info: true     # slot address and weight on each photo
```

Turn on `show_filenames` when you're trying to identify which photo is where.
Turn off `show_slot_info` once you're happy with the layout and just want a
clean preview.

---

## Photo index (appendix)

The template can append a photo index at the end of the book — a compact
reference listing every photo with its group, timestamp, and a reference back to
its page position. Configure it under `config.book.appendix` in `{name}.yaml`:

```yaml
config:
  book:
    appendix:
      active: true
      columns: 7
      ref_mode: "positions"   # or "counter"
      label_title: "Photo Index"
      label_page: "Page"
```

| Setting            | Default                                | Effect                                         |
| ------------------ | -------------------------------------- | ---------------------------------------------- |
| `active`           | `false`                                | Enable the appendix                            |
| `columns`          | `7`                                    | Number of columns in the listing               |
| `ref_mode`         | `"positions"`                          | How photos are referenced (see below)          |
| `page_separator`   | `false`                                | Show a page-number header between pages        |
| `strip_timestamps` | `true`                                 | Try to strip leading timestamps from filenames |
| `label_title`      | `"Photo Index"`                        | Localization: Title text                       |
| `label_page`       | `"Page"`                               | Localization: "Page" label                     |
| `date_format`      | `"{day}. {month} {year} {hour}:{min}"` | Timestamp format                               |
| `date_months`      | `["Jan", …, "Dec"]`                    | Month abbreviations                            |

### Reference modes

**`"positions"`** (default) — Each photo is referenced as `page.slot`, e.g.
`2.3` means page 2, slot 3. No visual badge is added to the photos.

**`"counter"`** — Photos are numbered sequentially (1, 2, 3, …) and a small
badge with the number appears in the bottom-right corner of each photo in the
PDF.

---

## Going further

The template reads layout data from `{name}.yaml` via `#let data = yaml(…)`.

A few things to keep in mind:

- `is_final` controls preview vs. release mode. Use it to conditionally show or
  hide elements: `#if not is_final [Draft watermark]`.
- Image paths are resolved relative to the project root via `cache_prefix`.
  Preview images live in `.fotobuch/cache/{name}/preview/`, final images in
  `.fotobuch/cache/{name}/final/`.

If you want to start over with the default template, create a fresh project with
`fotobuch project new` and copy the generated `{name}.typ` back.
