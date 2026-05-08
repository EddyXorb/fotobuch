# Glossary

Domain-specific terms used throughout fotobuch. These definitions are the authoritative source (DDD ubiquitous language) — all code, documentation, and communication should use them consistently.

## Book layout solver

The MIP (Mixed-Integer Programming) optimiser that assigns photos to pages. Runs during a full rebuild (`RebuildAll`). Determines *which* photos go on *which* page; does not control the visual arrangement within a page.

## Page layout solver

The genetic algorithm (GA) that arranges photos within a single page. Runs during a targeted rebuild (`RebuildPages`) and after the book layout solver. Determines the size, position, and orientation of each slot on a page.

## Photo

A single image file imported into the project. Identified by a stable content hash (ID). Belongs to exactly one photo group.

## Photo group

A named collection of photos, typically corresponding to a source folder. Used to organise imports; the book layout solver respects group boundaries when distributing photos.

## Project

A single photo book. Stored as a folder of YAML files tracked by git. Contains the photo list, page layout, and configuration. Multiple projects can live in the same vault.

## Slot

A positioned, sized rectangle on a page that holds exactly one photo. The page layout solver decides the geometry of each slot.

## Vault

The folder on disk that contains one or more projects. fotobuch remembers the last-used vault and re-opens it on startup.

## Page mode

Per-page flag that controls how the page layout solver behaves. **Auto** mode lets the solver choose the layout freely; **Manual** mode locks the slot positions and sizes so the solver does not move them.

## Release build

A high-resolution PDF export at full print quality, as opposed to the low-resolution preview renders shown in the GUI. Produces `{project-name}_final.pdf` in the project folder.
