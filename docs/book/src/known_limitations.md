# Known Limitations

## Cover page

Cover support exists but is still rough around the edges. The solver treats the
cover as a regular page during the first build, so it places multiple photos on
it.

**Workaround** to get a single cover photo:

```bash
# 1. After first build, move all but one photo off the cover (page 0)
fotobuch page move 0:1.. to 0+

# 2. Swap the remaining photo for one you actually want as the cover
fotobuch page swap 0:0 4:2   # swap with slot 2 on page 4, for example

# 3. Or add a fresh photo and place it directly onto the cover
fotobuch add ../cover-photo.jpg
fotobuch place --into 0

# 4. Edit the YAML to fine-tune slot positions if needed
```

## What fotobuch deliberately does not do

- **No pixel-level placement.** The solver decides where photos go; you
  influence it through weights, groups, and rebuild commands.
- **No mixed page sizes** within one project.
- **No image editing** (cropping, colour correction, rotation). Prepare your
  photos beforehand.
- **No upload integration** with print services — fotobuch stops at the PDF.
