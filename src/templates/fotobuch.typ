#let is_final = false
#let show_image_captions = true
#let show_borders = true
#let data = yaml("{name}.yaml")

// Cache-Pfad je nach Modus
#let cache_prefix = if is_final {
  ".fotobuch/cache/{name}/final/"
} else {
  ".fotobuch/cache/{name}/preview/"
}
// Seitengröße aus YAML inkl. Beschnitt / Bleed
#let bleed = data.config.book.bleed_mm * 1mm
#let margin = data.config.book.margin_mm * 1mm
// Seitengröße aus YAML
#set page(
  width: data.config.book.page_width_mm * 1mm + 2 * bleed,
  height: data.config.book.page_height_mm * 1mm + 2 * bleed,
  margin: bleed + margin,
)

// Draw border overlays for bleed and margin
#let draw_borders() = [
  // Red Bleed rectangle (outer boundary)
  #place(top + left, dx: -(bleed / 2 + margin), dy: -(bleed / 2 + margin), rect(
    width: data.config.book.page_width_mm * 1mm + bleed,
    height: data.config.book.page_height_mm * 1mm + bleed,
    stroke: red + bleed,
    fill: none,
  ))
  // Blue Margin rectangle (inner boundary)
  #place(top + left, dx: -margin / 2, dy: -margin / 2, rect(
    width: data.config.book.page_width_mm * 1mm - margin,
    height: data.config.book.page_height_mm * 1mm - margin,
    stroke: (paint: blue, thickness: margin),
    fill: none,
  ))
]



// Seiten rendern
#for page_data in data.layout [
  // Border Overlays
  #if show_borders [
    #draw_borders()
  ]

  // Content
  #for (i, slot) in page_data.slots.enumerate() [
    #let photo_id = page_data.photos.at(i, default: none)
    #if photo_id != none [
      #place(top + left, dx: slot.x_mm * 1mm, dy: slot.y_mm * 1mm, image(
        cache_prefix + photo_id,
        width: slot.width_mm * 1mm,
        height: slot.height_mm * 1mm,
        fit: "cover",
      ))
      #if show_image_captions [
        #place(
          top + left,
          dx: slot.x_mm * 1mm,
          dy: (slot.y_mm + slot.height_mm + 1) * 1mm,
          text(size: 8pt, photo_id.split("/").last()),
        )
      ]
    ]
  ]

  // Preview: Wasserzeichen
  #if not is_final [
    #place(center + horizon, rotate(-30deg, text(size: 120pt, fill: rgb("#00000055"), weight: "bold")[PREVIEW]))
  ]

  #pagebreak()
]


