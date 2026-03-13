# TODOs

To be done in this order

## To make is usable

- [x] typst template's width and height must be increased by the bleed and the pagelayout needs to be shifted towards (bleed,bleed) for x,y in to_page_layout.
- [ ] images should be ordered according to timestamp also *within* the same page. This needs to be considered within the ga_solver by introducing a penalty for photos being on the wrong side of the page (it should have linear complexity to calculate). This can be achieved by calculating the cumsum of the photo weight of each page and transform that linearly  for each photo to a x-coordinate and divide that by the canvas width (to scale it to 1);. Then look at each photo's center x, transform that to the [0,1] range as before and penalize the difference between actual A and wanted W coordinate by (A-X)^2, adding the penalty for each photo and divide that by the number of photos. This should then also get a new weight "in_page_order_deviation" or similar.
- [ ] Add should be able to handle single files too/groups of files, instead of ONLY whole dirs
- [ ] history outputs whole history, but should per default only be the last 5 (configurable via -n NR)
- [ ] images are rotated wrongly when read; probably due to the way how we read height and width (should take exif information)
- [ ] have a typst template that creates an image appendix with group name, time and date of each photo ( sorted by groups) and referenced either by a small counter subtext for each photo or without the subtext by lexicographic ordering of the upper left edge of each image (configurable in the template) 
- [ ] add a --unplaced flag to remove to remove all photos not in the layout
- [x] it should be possible to enable subtexts for each image during preview that shows the image id, should be disabled if is_final=true


## to improve it further


- [ ] make sure rebuild --page [nr] creates always a new layout that is different from the ones before (check git history) and make a configurable lookback with default 5. In case the solver does not generate a new layout restart it up to 10 times (unless it takes more than 200 ms to build the page) until a new layout is created. if not, ignore the lookbackrule
- [ ] make the genetic algorithm prune equal individuals to keep the genpool diverse; once done, output not only one layout but the best x layouts; this comes in handy when rebuilding a single page and we want a new layout than before.
- [ ] improve mutator of pagelayoutsolver: it should switch only leafes with different aspect ratios
- [ ] log* zu gitignore hinzufügen
- [ ] sort_key in groups should be checked to be unique to avoid randomness; to obtain a better key go into the folder of the group and take the first timestamp of all photos (as is done when no timestamp is in the group-name)