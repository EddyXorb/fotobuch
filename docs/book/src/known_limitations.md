# Known Limitations

## Cover page

Set `config.book.cover.mode` to a structured value (e.g. `front`, `spread-full`,
`split`) to get a deterministic, photo-count-aware cover layout without touching
the GA solver.  See [Cover modes](configuration.md#cover-modes) in the
configuration reference.

The default mode is `free`, which lets the GA solver treat the cover like any
other page.  If you use `free` mode after the first build you may need to
manually reassign photos:

```bash
# Move all but one photo off the cover (page 0)
fotobuch page move 0:1.. to 0+
fotobuch rebuild --page 0
```

## What fotobuch deliberately does not do

- **No pixel-level placement.** The solver decides where photos go; you
  influence it through weights, groups, and rebuild commands.
- **No mixed page sizes** within one project.
- **No image editing** (cropping, colour correction, rotation). Prepare your
  photos beforehand.
- **No upload integration** with print services — fotobuch stops at the PDF.
