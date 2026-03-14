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
- [ ] prüfen ob das generierte pdf wirklich die mediabox/bleedbox/targetbox so gesetzt hat, wie indesign das machen würde entsprechend den anforderungen für saal digital


## to improve it further


- [ ] automatically increase image weight according to xmp-rating. Low rating = smaller
- [ ] make sure rebuild --page [nr] creates always a new layout that is different from the ones before (check git history) and make a configurable lookback with default 5. In case the solver does not generate a new layout restart it up to 10 times (unless it takes more than 200 ms to build the page) until a new layout is created. if not, ignore the lookbackrule
- [ ] make the genetic algorithm prune equal individuals to keep the genpool diverse; once done, output not only one layout but the best x layouts; this comes in handy when rebuilding a single page and we want a new layout than before. → [Design: Population Diversity](docs/design/page_layout_solver_genetic_algorithm/population_diversity.md)
- [ ] improve mutator of pagelayoutsolver: it should switch only leafes with different aspect ratios
- [ ] log* zu gitignore hinzufügen
- [ ] sort_key in groups should be checked to be unique to avoid randomness; to obtain a better key go into the folder of the group and take the first timestamp of all photos (as is done when no timestamp is in the group-name)
- [ ] Verify that each image has a colour space and if not, set it for missing ones with a default that makes sense when creating the photo cache
  - Olympus OMD-EM1 JPEGs taggen den Farbraum nur im EXIF (`ColorSpace=1`=sRGB), betten aber kein ICC-Profil ein
  - Logik: EXIF `ColorSpace==1` → sRGB ICC einbetten; `ColorSpace==65535` → AdobeRGB ICC einbetten; ICC bereits vorhanden → nichts tun
  - Saal Digital unterstützt sRGB, AdobeRGB, ProPhoto RGB mit ICC-Farbmanagement; sRGB ist der sichere Default
  - Rust: `img_parts` für ICC-Chunks lesen/schreiben, image crate mit decoder und für EXIF-Tag, ICC-Profile als statische Bytes einbetten (~3KB) -> klären