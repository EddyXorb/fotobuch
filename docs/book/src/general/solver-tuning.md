# Solver Tuning

fotobuch has two solvers: the **book layout solver** (MIP + local search) distributes photos across pages; the **page layout solver** (genetic algorithm) arranges photos within each page. Most problems can be fixed by changing a few values in the YAML.

---

## Wrong page count

| Symptom | Fix |
| ------- | --- |
| Too many pages | Lower `book_layout_solver.page_target` |
| Too few pages | Raise `book_layout_solver.page_target` |
| Solver ignores the target | Raise `book_layout_solver.weight_pages` (default `5.0`) |
| Groups split across pages | Raise `book_layout_solver.weight_split` (default `10.0`) |

Always keep `page_max` a few pages above `page_target` so the solver can use an extra page when that improves the layout.

---

## Solver too slow

The total timeout (`search_timeout`, default 30 s) covers both the MIP and the local search phase:

```yaml
book_layout_solver:
  search_timeout:
    secs: 60
    nanos: 0
  enable_local_search: false   # skip local search for a faster (coarser) result
```

For books with 300+ photos the problem is automatically split into sub-problems. Raise `max_photos_for_split` to prevent splitting if you prefer one global solve.

---

## Poor single-page layouts

| Symptom | Fix |
| ------- | --- |
| Much white space | Raise `page_layout_solver.weights.w_coverage` (default `1.0`) |
| Photo sizes don't match weights | Raise `page_layout_solver.weights.w_size` (default `0.2`) |
| Chronological order wrong | Set `page_layout_solver.enforce_order: true` (default) |
| Same bad layout every rebuild | `fotobuch rebuild --page N` changes the seed automatically |

---

## Quick checklist

1. Set `page_target` to the desired page count.
2. Groups splitting? → raise `weight_split`.
3. Build too slow? → raise `search_timeout`, or set `enable_local_search: false`.
4. One bad page? → `fotobuch rebuild --page N` (picks a new random seed).
