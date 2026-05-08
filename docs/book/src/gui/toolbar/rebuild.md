# Rebuild Layout

The **Rebuild** button (keyboard shortcut `R`) asks fotobuch to re-run the layout solver for the selected pages.

## Targeted rebuild — pages selected

Select one or more pages in the Nav Strip or Central Panel, then press `R`. The page layout solver (genetic algorithm) runs again for each selected page, trying to find a better arrangement of the photos already assigned to those pages. Photo assignments between pages are **not** changed.

The button label shows how many pages will be rebuilt.

## Full rebuild — nothing selected

Press `R` with no pages selected and confirm when prompted. fotobuch first runs the book layout solver (MIP) to redistribute **all** photos across pages from scratch, then re-runs the page layout solver for every page. Use this when you want to start over entirely or when you have added or removed a significant number of photos.

## Note on optimality

The solvers are not exhaustive. A rebuild may produce the same layout as before if the current arrangement is already optimal or near-optimal given the current settings.
