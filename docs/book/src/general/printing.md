# Printing & Export

## General checklist

Before uploading your PDF, verify these things:

- Trigger a **release build** — only the release PDF has full 300 DPI.
  **Do NOT upload the preview PDF** to a print service; it will look blurry.
  - GUI: click **Release** in the toolbar (or press `Ctrl+Shift+B`)
  - CLI: `fotobuch build release`
- Check the output for **DPI warnings** (photos that are too small for their
  slot will be listed)
- Open the final PDF (`{name}_final.pdf`) and spot-check a few pages

## Saal Digital

fotobuch generates PDFs that meet
[Saal Digital's](https://www.saal-digital.com/) technical requirements out of
the box:

- **Bleed:** 3 mm on all sides (configurable via `config.book.bleed_mm` in the
  YAML)
- **PDF boxes:** MediaBox, TrimBox, and BleedBox are set correctly — matching
  what InDesign would produce
- **Resolution:** 300 DPI for the release build (configurable via
  `config.book.dpi`)

I tested the output with Saal Digital's online preview tool and it passed without any issues. You can upload the PDF directly to their website for printing. The only issue I had sometimes was that I did not know their spine widths exactly so I had to tweak the `config.book.spine_mm` setting a few times to get the preview to look right. If you know your spine width in advance, set it in the config before building and the preview should be correct on the first try.
I printed 7 photo books using *fotobuch* and I am quite happy with the results.

## Other print services

Most print-on-demand services accept standard PDF with bleed. Adjust
`config.book.bleed_mm` if your provider requires a different bleed size.
The default 3 mm works for the majority of European providers.

Some services lay out inner pages as facing pages and expect **no bleed on the
inside (spine) edge**, e.g. CEWE (3 mm outside/top/bottom, 0 mm at the spine).
For these, set `config.book.bleed_inside_mm: 0`. A 290 × 290 mm book then gets
293 × 296 mm inner pages whose TrimBox reaches the page edge on the spine side.

Besides editing the YAML, you can set the value:

- when creating the project: `fotobuch project new mybook --width 290 --height 290 --bleed 3 --bleed-inside 0`
  (also works with `fotobuch init`)
- in an existing project: `fotobuch config set book.bleed_inside_mm 0`; set it to
  `null` to go back to `bleed_mm` on all sides
- in the GUI: the *Bleed inside (mm)* field of the *New Project* dialog (empty
  means the same as *Bleed*), or `book.bleed_inside_mm` in the *Config* window
  (enter `null` to unset it)

After changing it in an existing project, run `fotobuch build` (or rebuild in
the GUI) to get the new PDF.

You do not set anything per page. fotobuch derives the side of each page from
its position and from `config.book.first_page_side`. Counting inner pages from 1
like printed page numbers (not the 0-based layout index), with the default
`first_page_side: right` pages 1, 3, 5, … are right-hand pages (spine on the
left) and pages 2, 4, 6, … are left-hand pages (spine on the right). The cover
does not count as an inner page and keeps the same bleed on all sides.

`right` fits products whose inside covers stay unprinted: the first inner page
faces the inside of the front cover, as with CEWE. If your product also prints
the inside of the front cover, the first inner page is a left-hand page. Set
`first_page_side: left` for this, and all sides swap:

- when creating the project: `--first-page-side left` (`project new` and `init`)
- in an existing project: `fotobuch config set book.first_page_side left`
- in the GUI: *First page* in the *New Project* dialog, or
  `book.first_page_side` in the *Config* window

Projects created before this option existed still use the old `.typ` template.
That template does not know the option, and the build stops with an error as
soon as `bleed_inside_mm` differs from `bleed_mm`. To
fix this, copy `<name>.typ` from a newly created project with the same name over
the old one (the template only depends on the project name), then re-apply any
edits you made to the old template.
