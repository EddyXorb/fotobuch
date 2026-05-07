# Central Panel

The central panel is the main editing canvas. It shows all pages in a vertical scroll column.

## Canvas and zoom

- **Ctrl+scroll** — zoom in/out around the cursor.
- **Ctrl+0** — fit page width to the available canvas.
- Zoom is stored per-session and does not affect the output.

## Page image

Each page displays a live preview rendered by the Typst compiler. While a re-render is pending the page shows a grey overlay with a spinner icon.

## Page HUD

A small heads-up display floats below each page. It appears faintly at rest and expands on hover.

| Element | Action |
|---------|--------|
| Page number | Display only |
| Mode pill (✦ AUTO / ✋ MANUAL) | Click to toggle Auto ↔ Manual |
| ↻ button | Rebuild this page |
| ✕ button | Delete this page |

## Drop zones

A drop zone appears between pages (and at the top/bottom of the list) when you are dragging photos from the Pool. Dropping here creates a new page and places the dragged photos onto it.

## Slot overlays

When the cursor is over a page the individual photo slots are outlined. Hovering a slot highlights it; clicking selects it (add more with Ctrl+click; extend a range with Shift+click).

Selected slots can be:
- **Deleted** — `Delete` key unplaces photos from the selected slots.
- **Weight-adjusted** — `W` opens the weight slider for fine-grained area control.

## Manual mode

In Manual mode each slot shows a yellow ✦ handle in the SE corner. Right-mouse-drag the handle to resize, or right-mouse-drag the slot body to move. A live preview outline follows the pointer while dragging.
