# TODOs

To be done in this order

## To make is *usable*

- [x] typst template's width and height must be increased by the bleed and the pagelayout needs to be shifted towards (bleed,bleed) for x,y in to_page_layout.
- [x] images should be ordered according to timestamp also *within* the same page → [Design: In-Page Ordering via DFS-Indexing](docs/design/page_layout_solver_genetic_algorithm/in_page_ordering_improvement.md)
- [x] Add should be able to handle single files too/groups of files, instead of ONLY whole dirs
- [x] add output to mip solver and configure timeout and gap to optimum as well as activate parallelism for that
- [x] test that add readds photos if the soure was recreated e.g. with lightroom. Could become a --update flag for subcommand "add" or similar
- [x] history outputs whole history, but should per default only be the last 5 (configurable via -n NR)
- [x] images are rotated wrongly when read; probably due to the way how we read height and width (should take exif information)
- [x] have a typst template that creates an image appendix with group name, time and date of each photo ( sorted by groups) and referenced either by a small counter subtext for each photo or without the subtext by lexicographic ordering of the upper left edge of each image (configurable in the template)
- [x] add a --unplaced flag to remove all photos not in the layout
- [x] it should be possible to enable subtexts for each image during preview that shows the image id, should be disabled if is_final=true
- [x] add --filter to "add" that works the same as --filter for remove
- [x] Seitenzuteilung für große Instanzen verbessern. Rückfallheuristik verwenden zur Not, aber auch vorher probieren das Problem in x Teilprobleme zu zerlegen, mit je eignenen Parametern die dem Mip dann sequenziell übergeben werden. Ein trigger für die teilung könnte das überschreiten einer maximalzahl an bildern sein (default: mit 100 bildern). Trennung erfolgt nicht zwangsläufig an gruppengrenzen, aber bevorzugt (dafür darf die teilungsgrenze um 5 abweichen. Die Teilproblemparameter bleiben identisch mit dem interschied der max-page und target-page parameter, die sich so aufteilen, dass die summe der teilproblemparameter den ursprünglichen parametern entspricht. der timeout verteilt sich auch gleichmäßig über alle aufrufe. die trennungsparameter sollen in die BookLayojtSolverconfig aufgenommen werden.
- [x] prüfen ob das generierte pdf wirklich die mediabox/bleedbox/targetbox so gesetzt hat, wie indesign das machen würde entsprechend den anforderungen für saal digital
- [x] make DPI of final configurable in yaml, as well as jpg-quality for both preview and final
- [x] order the slots/photos in layout according to reading order (to make the appendix work later)

## to improve it further

- [ ] Coloured terminal output (`owo-colors` or `console` crate): success green, errors red,
      warnings yellow, highlighted paths — makes the CLI feel polished and professional
- [ ] Progress bars for solver & build (`indicatif` crate): "Building page 3/24…", spinner
      during MIP solve. Eliminates "is it stuck?" anxiety, especially for release builds.
- [ ] Richer `fotobuch status` output: human-readable summary showing project name, group/photo/page
      counts, cover state, unplaced photos, last build type and age — all at a glance
- [ ] reevaluate cli commands for page (maybe remove "to" in page move)
- [ ] page numbering as option in template
- [ ] preview config is not applied in template - remove or fix it
- [ ] cli should have a command to output the template again
- [ ] `fotobuch config` should output also cover options
- [ ] remove max_coverage_cost parameter, as it is not used anymore
- [ ] new should be clearer where it creates the folder (and that its created at all). The --parent-dir option is not explained clearly enough. It should be clear that its not necessary to create a new folder for a new project, but that it can be done within an exisitng fotobuch-repo
- [ ] automatically increase image weight according to xmp-rating. Low rating = smaller
- [ ] make sure rebuild --page [nr] creates always a new layout that is different from the ones before (check git history) and make a configurable lookback with default 5. In case the solver does not generate a new layout restart it up to 10 times (unless it takes more than 200 ms to build the page) until a new layout is created. if not, ignore the lookbackrule
- [ ] make the genetic algorithm prune equal individuals to keep the genpool diverse; once done, output not only one layout but the best x layouts; this comes in handy when rebuilding a single page and we want a new layout than before. → [Design: Population Diversity](docs/design/page_layout_solver_genetic_algorithm/population_diversity.md) -> tried simple deduplicatation, no change in quality
- [ ] improve mutator of pagelayoutsolver: it should switch only leafes with different aspect ratios
- [x] log* zu gitignore hinzufügen
- [ ] sort_key in groups should be checked to be unique to avoid randomness; to obtain a better key go into the folder of the group and take the first timestamp of all photos (as is done when no timestamp is in the group-name)
- [ ] Verify that each image has a colour space and if not, set it for missing ones with a default that makes sense when creating the photo cache
  - Olympus OMD-EM1 JPEGs taggen den Farbraum nur im EXIF (`ColorSpace=1`=sRGB), betten aber kein ICC-Profil ein
  - Logik: EXIF `ColorSpace==1` → sRGB ICC einbetten; `ColorSpace==65535` → AdobeRGB ICC einbetten; ICC bereits vorhanden → nichts tun
  - Saal Digital unterstützt sRGB, AdobeRGB, ProPhoto RGB mit ICC-Farbmanagement; sRGB ist der sichere Default
  - Rust: `img_parts` für ICC-Chunks lesen/schreiben, image crate mit decoder und für EXIF-Tag, ICC-Profile als statische Bytes einbetten (~3KB) -> klären


## `page` / `unplace` commands

> Design: [docs/design/cli/page.md](docs/design/cli/page.md)

- [x] Implement lib types: `PagesExpr`, `SlotExpr`, `Src`, `DstMove`, `DstSwap`, `PageMoveCmd` in `src/commands/page.rs`
- [x] Implement `ValidationError`, `PageMoveError`, `PageMoveResult` types
- [x] Implement `execute_unplace` (removes photos by slot from layout)
- [x] Implement `execute_move` — Move variant (`->`)
- [x] Implement `execute_move` — Swap variant (`<>`)
- [x] Implement `execute_split` (shortcut: move PAGE:SLOT.. -> PAGE+)
- [x] Implement `execute_combine` (shortcut: merge pages, delete empties)
- [x] Implement Lexer in `src/cli/page.rs` (Token enum + tokenize fn)
- [x] Implement Parser in `src/cli/page.rs` (builds AST from tokens)
- [x] Wire up CLI: add `Unplace` and `Page` subcommands to `src/cli.rs`
- [x] Tests for all lib execute_* functions
- [x] Tests for lexer and parser

## Workflow improvements (found during book review)

- [ ] Cover setup is too manual (3–4 commands + YAML edit). Solution: a `mode`
      field in `config.book.cover` with fixed slot layouts — see `COVER_WORKFLOW_IMPROVEMENT.md`.
- [ ] **Generalise page layout modes to inner pages.** The cover `mode` concept
      (fixed slot positions instead of GA-solver) could apply to any page via
      `page mode <idx> <mode>`. The mode is then saved as optional child of each page in the yaml.
      An additional mode should support a configurable **inner margin** (gutter) for non-lay-flat bindings — the solver would keep
      photos away from the spine side of each page. This avoids photos disappearing
      into the binding on double-page spreads. Modes for inner pages `spread-free`, `spread-single`, `split-single`
      , `gutter:<mm>` (solver respects a keep-out zone on the spine side).
- [ ] First `build` auto-places all photos, but subsequent builds require
      explicit `fotobuch place`. This asymmetry is surprising. Consider
      auto-placing in `build` or warning loudly about unplaced photos.
- [ ] `build --pages` (limit rendering) vs `rebuild --page` (re-solve) have
      confusingly similar names. Consider unifying or renaming.

## Internal todos

- [ ] clean up the builder-section and have a new wrapper that calls the others, but takes care to build the pdf and get the correct BookLayoutConfig for further processing.