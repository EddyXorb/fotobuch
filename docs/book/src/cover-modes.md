# Cover Modes — Visual Guide

Each cover mode determines how photos are positioned and sized on the cover. The examples below show the result of each mode with a sample photo.

---

## Mode: `front`

A single photo on the front panel, with its aspect ratio preserved and centred.

![front mode example](/cover-modes/front.svg)

**Use when:** You have one focal photo that should appear on the front panel without cropping.

---

## Mode: `front-full`

A single photo fills the entire front panel (may crop to fit).

![front-full mode example](/cover-modes/front-full.svg)

**Use when:** You want maximum visual impact and don't mind if the photo is cropped at the edges.

---

## Mode: `back`

A single photo on the back panel, with its aspect ratio preserved and centred.

![back mode example](/cover-modes/back.svg)

**Use when:** You want a focal photo on the back cover without cropping.

---

## Mode: `back-full`

A single photo fills the entire back panel (may crop to fit).

![back-full mode example](/cover-modes/back-full.svg)

**Use when:** You want a visually striking back cover and don't mind cropping.

---

## Mode: `spread`

A single photo spans the full spread (front, spine, and back), with its aspect ratio preserved and centred.

![spread mode example](/cover-modes/spread.svg)

**Use when:** You have a panoramic or very wide photo that should wrap around the entire cover.

---

## Mode: `spread-full`

A single photo fills the full spread without cropping space for the spine (may crop the photo).

![spread-full mode example](/cover-modes/spread-full.svg)

**Use when:** You want a panorama to cover the entire spreads seamlessly.

---

## Mode: `split`

Two photos: slot 0 goes on the front panel, slot 1 on the back panel. Both have their aspect ratios preserved and are centred.

![split mode example](/cover-modes/split.svg)

**Use when:** You want two distinct photos — one for each side — without cropping.

---

## Mode: `split-full`

Two photos: slot 0 fills the front panel, slot 1 fills the back panel (each may crop independently).

![split-full mode example](/cover-modes/split-full.svg)

**Use when:** You want maximum visual impact on both front and back, with each photo filling its space.

---

## Mode: `free`

The genetic algorithm solver optimises photo placement freely without constraints. Use any number of photos.

**Use when:** You want the solver to arrange photos however produces the best layout (most common choice).

---

## Choosing a Mode

| Scenario | Recommended Mode |
|----------|------------------|
| One focal photo, no cropping | `front` or `back` |
| One photo, fill the space | `front-full` or `back-full` |
| Panoramic photo across full spread | `spread` or `spread-full` |
| Two photos (front & back) | `split` or `split-full` |
| Multiple photos, best layout | `free` |
