# Printing & Known Limitations

## Exporting for Saal Digital

fotobuch generates PDFs that meet Saal Digital's technical requirements out of
the box:

- **Bleed:** 3 mm on all sides (configurable via `bleed_mm` in `fotobuch.yaml`)
- **PDF boxes:** MediaBox, TrimBox, and BleedBox are set correctly — matching
  what InDesign would produce for a Saal Digital upload
- **Resolution:** 300 DPI for `build release` (configurable via `dpi`)
- **Colour space:** sRGB (embedded ICC profile)

### Export steps

1. Run `fotobuch build release` to generate `release.pdf`
2. Upload `release.pdf` directly to Saal Digital

No further processing needed.

### Cover export

If your project has an active cover (`cover.active: true` in `fotobuch.yaml`),
the cover is exported as a separate PDF spread (front + spine + back on one
page). Upload it separately from the inner pages PDF.

---

## Known Limitations

### Cover photo placement

Adding a cover currently requires manually editing `fotobuch.yaml` to position
photos on the front and back panels without overlapping the spine. The solver
also distributes regular photos onto the cover page during the first build.

**Workaround:** after `fotobuch build`, use `fotobuch unplace` to remove photos
from the cover page, then manually add cover photos via `fotobuch add` and
`fotobuch place --into 0`.

This limitation is tracked and will be addressed in a future release.

### `page move` syntax

The command `fotobuch page move 3:2 to 5` contains a `to` keyword that feels
unexpected compared to most CLI tools. This will be revisited.

### No page numbering in template

The default Typst template does not render page numbers. This can be added
manually in the `.typ` file.

### Preview config not applied in template

The `preview` section of `fotobuch.yaml` (show filenames, show page numbers)
is not yet applied by the Typst template. Filename captions visible in the
preview are a known gap.

---

## What fotobuch deliberately does not do

- **No manual placement of individual photos at arbitrary coordinates.**
  The solver decides placement; you influence it through weights, groups, and
  rebuild commands.
- **No support for mixed page sizes** within one project.
- **No built-in image editing** (colour correction, cropping, rotation).
  Prepare your photos in Lightroom or similar before adding them.
- **No upload integration** with print services. fotobuch stops at the PDF.
