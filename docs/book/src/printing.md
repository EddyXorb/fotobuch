# Printing & export

## Exporting for Saal Digital

fotobuch generates PDFs that meet e.g. [Saal Digital's](https://www.saal-digital.com/) technical requirements out of the box:

- **Bleed:** 3 mm on all sides (configurable via `config.book.bleed_mm` in yaml-file)
- **PDF boxes:** MediaBox, TrimBox, and BleedBox are set correctly — matching
  what InDesign would produce
- **Resolution:** 300 DPI for `build release` (configurable via `config.dpi`)
