#let is_final = false
#let data = yaml("{name}.yaml")

// Cache-Pfad je nach Modus
#let cache_prefix = if is_final {
  ".fotobuch/cache/{name}/final/"
} else {
  ".fotobuch/cache/{name}/preview/"
}

// Convert EXIF orientation tag to rotation angle for Typst
// EXIF: 1=normal, 6=90°CW, 8=270°CW
// Typst rotate(): positive = counter-clockwise
#let get_rotation_angle(orientation) = {
  if orientation == 6 { -90deg }
  else if orientation == 8 { 90deg }
  else if orientation == 3 { 180deg }
  else { 0deg }
}

// Seitengröße aus YAML inkl. Beschnitt / Bleed
#let bleed = data.config.book.bleed_mm * 1mm
#set page(
  width: data.config.book.page_width_mm * 1mm + 2 * bleed,
  height: data.config.book.page_height_mm * 1mm + 2 * bleed,
  margin: data.config.book.margin_mm * 1mm + bleed,
)

// Seiten rendern
#for page_data in data.layout [
  #page[
    #for (i, slot) in page_data.slots.enumerate() [
      #let photo_id = page_data.photos.at(i, default: none)
      #if photo_id != none [
        #place(
          top + left,
          dx: slot.x_mm * 1mm,
          dy: slot.y_mm * 1mm,
          image(
            cache_prefix + photo_id,
            width: slot.width_mm * 1mm,
            height: slot.height_mm * 1mm,
            fit: "cover",
          )
        )
      ]
    ]

    // Preview: Wasserzeichen
    #if not is_final [
      #place(
        center + horizon,
        rotate(
          -30deg,
          text(
            size: 48pt,
            fill: rgb(0, 0, 0, 40),
            weight: "bold",
          )[PREVIEW]
        )
      )
    ]
  ]
]
