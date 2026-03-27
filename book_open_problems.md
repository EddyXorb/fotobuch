# Book: Open Problems

Issues found reading the book as a newcomer who knows nothing about the project.

## Critical: Contradictions

1. **Slot numbering: 0-based vs 1-based**
   - `quickstart.md` line 96: "slots are numbered from 1, left-to-right, top-to-bottom"
   - `commands.md` line 47: "Pages and slots are numbered from 0"
   - Code truth: **both are 0-based**. The quickstart is wrong.

2. **Release PDF filename**
   - `quickstart.md` line 132: "writes `release.pdf`"
   - Code truth: writes `{project_name}_final.pdf`

3. **Cover workaround uses wrong page numbers**
   - `known_limitations.md`: `fotobuch page move 1:2.. to 1+`
   - Cover = page 0 (0-based). Should be `0:1.. to 0+`.

4. **YAML field paths are inconsistent**
   - `printing.md`: references `config.book.bleed_mm` and `config.dpi`
   - `commands.md` table: shows flat names `bleed_mm`, `dpi`, `solver.page_target`
   - Code truth: YAML nesting is `book.bleed_mm`, `book.dpi`, `book_layout_solver.page_target`
   - None of the three match each other.

## Major: Missing concepts

5. **placed vs unplaced lifecycle never explained**
   - `add` imports photos into the project (unplaced).
   - First `build` places all photos automatically.
   - Later `add` creates new unplaced photos that need `fotobuch place` before `build`.
   - This lifecycle is invisible in the book. A user adding photos after the first build will be confused.

6. **Groups never explained**
   - Each added folder becomes a "group". Groups keep photos together on pages.
   - The solver limits groups per page and enforces minimum photos per group split.
   - The book mentions "group" only in passing during `add`. The reader has no idea what this means for the layout.

7. **Multi-project model not explained**
   - Projects live on separate git branches (`fotobuch/<name>`) in the same repo.
   - `project switch` is a git checkout. This is completely non-obvious.
   - Quickstart says "creates a folder" but doesn't explain the branch model.

8. **`rebuild` vs `build` distinction unclear**
   - When to use `rebuild --page 6` vs `build --pages 6`? Both exist.
   - `rebuild` re-runs the genetic algorithm. `build --pages` limits rendering?

9. **Weight concept needs more explanation**
   - What does weight=2 actually do? Makes the photo take ~2x the area?
   - Can be set during `add` (for all photos) or per-slot with `page weight`.

## Minor: Confusing or incomplete

10. **"mixed-integer program" mentioned in installation.md**
    - Not explained. Users don't need to know this. It's an implementation detail.
    - "cmake for Highs" — HiGHS is a solver library, but nobody knows this from the text.

11. **Cover feature poorly documented**
    - Only appears in "known limitations" — not in quickstart or commands.
    - `--with-cover` flag exists on `project new` but isn't mentioned in the walkthrough.
    - Cover config (spine, front/back width) completely undocumented.

12. **`margin_mm` exists as `--margin-mm` CLI flag** but not documented in project new walkthrough.

13. **`--filter-xmp` example is overloaded**
    - One-liner in quickstart has a huge inline comment and uses `--weight` too.

14. **Known limitations cover workaround is a run-on sentence mess**
    - Grammar issues, hard to parse, wrong page numbers.

15. **`page move` help text in reference says `e.g.: 3:2 -> 5`** but only `to` keyword works.

16. **Printing page is very thin** — only Saal Digital, 3 bullet points.

17. **No `--margin-mm` in project new reference** — Actually it IS in the CLI (confirmed in code) but the auto-generated reference doesn't show it? Need to check.
