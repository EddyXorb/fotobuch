# TODOs

To be done in this order

- [ ]  history outputs whole history, but should per default only be the last 5 (configurable via -n NR)
- [ ] images are rotated wrongly when read; probably due to the way how we read height and width (should take exif information)
- [ ] it should be possible to enable subtexts for each image during preview that shows the image id (configurable via variable in typst template and also via cli-flag on build (--annotate), which is incompatible with --release)
- [ ] have a typst template that creates an image appendix with group name, time and date of each photo ( sorted by groups) and referenced either by a small counter subtext for each photo or without the subtext by lexicographic ordering of the upper left edge of each image (configurable in the template) 
- [ ] images should be ordered according to timestamp also *within* the same page. This needs to be considered within the ga_solver by introducing a penalty for photos being on the wrong side of the page (it should have linear complexity to calculate). This can be achieved by calculating the cumsum of the photo weight of each page and transform that for each photo to a x-coordinate and divide that by the canvas width (to scale it to 1);. Then look at each photo's center x, transform that to the [0,1] range as before and penalize the difference between actual A and wanted W coordinate by (A-X)^2, adding the penalty for each photo and divide that by the number of photos. This should then also get a new weight "in_page_order_deviation" or similar.