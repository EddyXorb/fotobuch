# Drag Mode (Swap and Move)

The **⇄ Swap** and **→ Move** buttons control what happens when you drag an image with the right mouse button in the Central Panel. Toggle between them with the `M` key.

## Swap mode (⇄)

Dragging an image and dropping it on another **exchanges** the two photos.

**When both images have the same aspect ratio**, the exchange is immediate and both positions stay the same.

**When the images have different aspect ratios:**
- *Same page* — the swap is blocked. Auto mode re-sorts photos by aspect ratio when the page is recalculated, so the exchange would be undone immediately. Use Move mode to relocate photos between slots on the same page.
- *Different pages* — the swap goes through, but both affected pages may be recalculated to fit the new photo proportions, which can change the visual layout of those pages.

## Move mode (→)

Dragging an image and dropping it on another position **relocates** the dragged photo to that slot. The target slot's geometry is applied to the moved photo.

- Drop on a slot on a **different page** to move the photo there.
- Drop on the **Nav Bar** to move the photo to that page.
- Moving within the **same page** is only possible in Manual mode. In Auto mode, use Swap instead (if ratios match), or switch to Manual mode first.

## Which mode to use

- **Swap** for quick reordering of same-ratio images — page layout stays intact.
- **Move** to shift a photo to a completely different page or to reposition in Manual mode.
