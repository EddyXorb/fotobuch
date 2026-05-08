# CLI Concepts

## Slot addresses

A **slot address** identifies one or more placed photos on a specific page:

| Address    | Meaning                                                |
| ---------- | ------------------------------------------------------ |
| `3`        | All slots on page 3                                    |
| `3:2`      | Slot 2 on page 3                                       |
| `3:2..5`   | Slots 2 through 5 on page 3                            |
| `3:2..5,7` | Slots 2–5 and slot 7 on page 3                         |
| `4+`       | New page inserted after page 4 (move destination only) |

Use `fotobuch status <page>` to see which slot numbers are on a given page.

## Two ways to address photos

| Method           | Use case                                         | Examples                         |
| ---------------- | ------------------------------------------------ | -------------------------------- |
| **Filename / pattern** | Photos not yet placed, or matching by source path | `add`, `remove`, `place --filter` |
| **Slot address** | Photos already placed on a page                  | `page move`, `page swap`, `page weight` |

**Rule of thumb:** use filename patterns for *unplaced* photos (`add`, `place`,
`remove`); use slot addresses for *placed* photos (`page move`, `page swap`,
`page weight`, `unplace`).

## Build pipeline

```
fotobuch add     →  photos enter the project (unplaced)
fotobuch place   →  unplaced photos get assigned to pages
fotobuch build   →  solver optimises layout, renders preview PDF
fotobuch build release  →  renders final PDF at print resolution
```

The first `build` implicitly places all photos, so you can skip `place` on a
fresh project.

Every command that changes the layout creates a **Git commit** automatically.
Use `fotobuch undo` / `fotobuch redo` to navigate history, and
`fotobuch history` to see the log.
