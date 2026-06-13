# Solver Tuning

fotobuch has two solvers: the **book layout solver** (exact DP + local search) distributes photos across pages; the **page layout solver** (genetic algorithm) arranges photos within each page. Most problems can be fixed by changing a few values in the YAML.

Curious *how* these solvers work under the hood? See [How the Layout Engine Works](layout-engine.md).


## Quick checklist

1. Set `page_target` to the desired page count.
2. Groups splitting? → raise `weight_split`.
3. Build too slow? → raise `search_timeout`, or set `enable_local_search: false`.

---

## Wrong page count

| Symptom                   | Fix                                                                                                                 |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Too many pages            | Lower `book_layout_solver.page_target`                                                                              |
| Too few pages             | Raise `book_layout_solver.page_target`                                                                              |
| Solver ignores the target | Raise `book_layout_solver.weight_pages` (default `5.0`)                                                             |
| Groups split across pages | Raise `book_layout_solver.weight_split` (default `10.0`) and/or set `book_layout_solver.enable_local_search: false` |

Always keep `page_max` a few pages above `page_target` so the solver can use an extra page when that improves the layout significantly.

---

## Solver too slow

The page assignment phase uses an exact dynamic program that solves even
thousand-photo books in milliseconds, so it is no longer a bottleneck. The
`search_timeout` (default 30 s) now bounds only the local search phase:

```yaml
book_layout_solver:
  search_timeout:
    secs: 60
    nanos: 0
  enable_local_search: false   # skip local search for a faster (coarser) result
```

---

## Poor single-page layouts

| Symptom                         | Fix                                                           |
| ------------------------------- | ------------------------------------------------------------- |
| Much white space                | Raise `page_layout_solver.weights.w_coverage` (default `1.0`) |
| Photo sizes don't match weights | Raise `page_layout_solver.weights.w_size` (default `0.2`)     |
| Chronological order wrong       | Set `page_layout_solver.enforce_order: true` (default)        |

