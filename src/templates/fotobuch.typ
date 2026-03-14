// ===== user space: you can edit these flags here safely ===

#let is_final = false
#let show_image_captions_on_preview = true
#let show_borders_on_preview = true

// Anhang / Appendix settings
#let show_appendix = true
// "counter": sequential ref numbers shown as small badge on each photo in the main layout;
//            find a photo in the appendix by matching that number.
// "position": no badge shown on photos; appendix reference = "page.position",
//             where position is determined by lexicographic order of the upper-left corner
//             (top-to-bottom, left-to-right), e.g. "2.3" = page 2, 3rd photo.
#let appendix_ref_mode = "counter"

// ====== user space end, do not edit below this line if you don't know Typst well enough =======

#if is_final [
  #let show_image_captions_on_preview = false
  #let show_borders_on_preview = false
]
#let project_name = "{project_name}"
#let data = yaml(project_name + ".yaml")
#set text(font: "Libertinus Serif")
// Cache-Pfad je nach Modus
#let cache_prefix = if is_final {
  ".fotobuch/cache/" + project_name + "/final/"
} else {
  ".fotobuch/cache/" + project_name + "/preview/"
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

// ---- Reference label computation ----
// Builds photo_ref: photo_id -> label string
// Counter mode: labels are "1", "2", … in page/slot order.
// Position mode: labels are "page.pos" where pos is the 1-based index after
//               sorting slots lexicographically by (y_mm, x_mm).
#let photo_ref = (:)
#if appendix_ref_mode == "counter" {
  let n = 1
  for page_data in data.layout {
    for photo_id in page_data.photos {
      photo_ref.insert(photo_id, str(n))
      n = n + 1
    }
  }
} else {
  for (pi, page_data) in data.layout.enumerate() {
    let pairs = ()
    for (i, photo_id) in page_data.photos.enumerate() {
      pairs = pairs + ((photo_id, page_data.slots.at(i)),)
    }
    let sorted = pairs.sorted(key: p => (p.at(1).y_mm, p.at(1).x_mm))
    for (pos, p) in sorted.enumerate() {
      photo_ref.insert(p.at(0), str(pi + 1) + "." + str(pos + 1))
    }
  }
}

// Format ISO 8601 timestamp string → "YYYY-MM-DD HH:MM"
#let fmt_ts(ts) = {
  let s = str(ts)
  if s.len() >= 16 { s.slice(0, 10) + " " + s.slice(11, 16) } else { s }
}

// Seiten rendern
#for (page_index, page_data) in data.layout.enumerate() [

  // Border Overlays
  #if show_borders_on_preview [
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
      #if show_image_captions_on_preview [
        #place(
          top + left,
          dx: slot.x_mm * 1mm,
          dy: (slot.y_mm + slot.height_mm / 2) * 1mm,
          text(size: 8pt, photo_id.split("/").last(), white),
        )
      ]
      // Counter badge: bottom-right corner of each photo (counter mode only)
      #if show_appendix and appendix_ref_mode == "counter" [
        #let ref_label = photo_ref.at(photo_id, default: "")
        #if ref_label != "" [
          #place(
            top + left,
            dx: (slot.x_mm + slot.width_mm - 10) * 1mm,
            dy: (slot.y_mm + slot.height_mm - 7) * 1mm,
            box(
              fill: rgb("#00000099"),
              inset: (x: 1.5mm, y: 1mm),
              text(size: 7pt, fill: white, weight: "bold")[#ref_label],
            ),
          )
        ]
      ]
    ]
  ]

  // Preview: Wasserzeichen
  #if not is_final [
    #place(center + horizon, rotate(-30deg, text(size: 120pt, fill: rgb("#00000055"), weight: "bold")[PREVIEW]))
  ]

  #if page_index < data.layout.len() - 1 or show_appendix [#pagebreak()]
]

// ---- Anhang / Image Appendix ----
#if show_appendix [
  #set page(margin: 15mm)

  #text(size: 22pt, weight: "bold")[Anhang – Bildnachweis]
  #v(0.6em)
  #text(size: 9pt, fill: rgb("#666666"))[
    #if appendix_ref_mode == "counter" [
      Die Nummern entsprechen den kleinen Referenzzahlen auf den Fotos im Buch.
    ] else [
      Die Referenz zeigt Seite.Position (z.B. „2.3" = Seite 2, 3. Foto nach Lesereihenfolge von oben links).
    ]
  ]
  #v(1.5em)

  #for group_data in data.photos [
    #if group_data.files.len() > 0 [
      // Gruppenüberschrift
      #block(
        fill: rgb("#eeeeee"),
        inset: (x: 3mm, y: 2mm),
        width: 100%,
        text(size: 12pt, weight: "bold")[#group_data.group],
      )
      #v(0.4em)

      // Fotos sortiert nach Zeitstempel, 4 Spalten
      #let sorted_files = group_data.files.sorted(key: f => str(f.timestamp))
      #grid(
        columns: (1fr, 1fr, 1fr, 1fr),
        column-gutter: 3mm,
        row-gutter: 5mm,
        ..sorted_files.map(file => {
          let ref_label = photo_ref.at(file.id, default: "–")
          block(width: 100%, [
            #image(cache_prefix + file.id, width: 100%, height: 26mm, fit: "cover")
            #block(width: 100%, inset: (top: 1mm), [
              #box(
                fill: rgb("#333333"),
                inset: (x: 1.5mm, y: 0.7mm),
                text(size: 7pt, fill: white, weight: "bold")[#ref_label],
              )
              #h(1.5mm)
              #text(size: 7pt, fill: rgb("#444444"))[#fmt_ts(file.timestamp)]
            ])
          ])
        }),
      )
      #v(1.2em)
    ]
  ]
]
