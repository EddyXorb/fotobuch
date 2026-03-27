# Printing & Export

## General checklist

Before uploading your PDF, verify these things:

- Run `fotobuch build release` — only the release PDF has full 300 DPI, do **NOT UPLOAD THE PREVIEW PDF** to be printed, it will be ugly!
- Check the terminal output for **DPI warnings** (photos that are too small for
  their slot will be listed)
- Open the final PDF (`{name}_final.pdf`) and spot-check a few pages

## Saal Digital

fotobuch generates PDFs that meet
[Saal Digital's](https://www.saal-digital.com/) technical requirements out of
the box:

- **Bleed:** 3 mm on all sides (configurable via `config.book.bleed_mm` in the
  YAML)
- **PDF boxes:** MediaBox, TrimBox, and BleedBox are set correctly — matching
  what InDesign would produce
- **Resolution:** 300 DPI for `build release` (configurable via
  `config.book.dpi`)

## Other print services

Most print-on-demand services accept standard PDF with bleed. Adjust
`config.book.bleed_mm` if your provider requires a different bleed size.
The default 3 mm works for the majority of European providers.
