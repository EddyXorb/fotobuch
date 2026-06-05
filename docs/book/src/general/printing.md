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
