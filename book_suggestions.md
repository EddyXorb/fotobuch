# Book: Improvement Suggestions

## Structure changes

1. **Add a "Core Concepts" page** between Installation and Quickstart
   - Project = git repo with branches for multiple books
   - Photo lifecycle: add → (unplaced) → place/build → (placed on pages)
   - Groups: folders become groups, groups stay together on pages
   - Weights: relative area a photo occupies
   - Slot addressing: `page:slot` (both 0-based)
   - Cover: optional, page 0 when active

2. **Streamline the quickstart**
   - Fix all wrong numbers (slots 0-based, correct filename)
   - Add a "Step 2.5 — Adding more photos later" that explains `place`
   - Show `status` output after key steps so readers know what to expect
   - Keep it short but correct

3. **Split commands.md into subsections**
   - "Command Overview" (keep the table, fix it)
   - "Slot Addressing" (already exists, keep)
   - "YAML Configuration" → fix field paths, show actual YAML snippet

4. **Fix YAML config section completely**
   - Show a real YAML example with correct nesting (`book.`, `book_layout_solver.`, etc.)
   - Only document user-facing fields, skip solver internals
   - Remove reference to `docs/design/yaml-scheme.md` (dev doc)

5. **Expand printing page OR merge into quickstart step 5**
   - Add a "what to check before uploading" checklist
   - Fix the field path references

6. **Rewrite known limitations cover section**
   - Fix page numbers (0-based)
   - Break the run-on sentence into clear steps
   - Consider a proper "Cover setup" section instead of burying it in limitations

## Tone improvements

- The book is already fairly concise. Keep that.
- Add a light touch of personality in the intro and quickstart (not every page).
- Remove jargon where possible ("mixed-integer program" → just say "optimizer").
- Examples are more helpful than explanations — add more command examples.

## Things that should NOT be in the book

- Solver parameters (population_size, mutation_rate, etc.) — too advanced
- Internal architecture details
- Build-from-source cmake explanation beyond the minimum
