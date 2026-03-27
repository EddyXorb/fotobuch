# Known Limitations

## Cover photo placement

Adding a cover currently requires manually editing the yaml to position
photos on the front and back panels without overlapping the spine. The solver
also distributes regular photos onto the cover page during the first build.

**Workaround:** after `fotobuch build`, use `fotobuch page move 1:2.. to 1+` to move photos
from the cover page on a new page after the cover page, with just one photo remaining on the cover page (if you move all fotos, the cover page will be removed and the new page will become the cover page, resulting in effectively no change at all)
, then manually swap the remaning photo with one you already placed within the book or add a new photo with `fotobuch add ../new_photo.jpg` and place it then with `fotobuch place new_photo --into 0`.
Then go into the yaml file and adapt the slots to your needs.

## What fotobuch deliberately does not do

- **No manual placement of individual photos at arbitrary coordinates.**
  The solver decides placement; you influence it through weights, groups, and
  rebuild commands.
- **No support for mixed page sizes** within one project.
- **No built-in image editing** (colour correction, cropping, rotation).
  Prepare your photos in Lightroom or similar before adding them.
- **No upload integration** with print services. fotobuch stops at the PDF.