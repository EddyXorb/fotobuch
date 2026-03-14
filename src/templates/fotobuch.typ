// ===== user space: you can edit these flags here safely ===

#let is_final = false
#let show_image_captions_on_preview = true
#let show_borders_on_preview = true

// Anhang / Appendix settings
#let show_appendix = false
// "counter": sequential ref numbers shown as small badge on each photo in the main layout;
//            find a photo in the appendix by matching that number.
// "position": no badge shown on photos; appendix reference = "page.position",
//             where position is determined by lexicographic order of the upper-left corner
//             (top-to-bottom, left-to-right), e.g. "2.3" = page 2, 3rd photo.
#let appendix_ref_mode = "positions"

// ====== user space end, do not edit below this line if you don't know Typst well enough =======

#if is_final [
  #let show_image_captions_on_preview = false
  #let show_borders_on_preview = false
]
#let project_name = "{project_name"
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

#let try_strip_datetime_from(s) = {
  s.replace(regex("^\d{4}-\d{2}-\d{2}[@T]\d{6}[_-]?"), "")
}

// ---- Reference label computation ----
// Builds photo_ref: photo_id -> label string
// Counter mode: labels are "1", "2", … in page/slot order.
// Position mode: labels are "page.pos" where pos is the 1-based index after
//               sorting slots lexicographically by (y_mm, x_mm).

#let calc_photo_ref() = {
  let photo_ref = (:)
  if appendix_ref_mode == "counter" {
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
      for (pos, p) in pairs.enumerate() {
        photo_ref.insert(p.at(0), str(pi + 1) + "." + str(pos + 1))
      }
    }
  }
  photo_ref
}

// Format ISO 8601 timestamp → "1. Jan 10:10 Uhr"
#let fmt_ts_de(ts) = {
  let s = str(ts)
  if s.len() < 16 { return s }
  let monate = ("Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez")
  let year = s.slice(0, 4)
  let day = str(int(s.slice(8, 10)))
  let month = monate.at(int(s.slice(5, 7)) - 1)
  let hour = s.slice(11, 13)
  let min = s.slice(14, 16)
  day + ". " + month + " " + year + " " + hour + ":" + min + " Uhr"
}

// photo_id → timestamp lookup aus data.photos
#let photo_ts = {
  let m = (:)
  for group in data.photos {
    for file in group.files {
      m.insert(file.id, file.timestamp)
    }
  }
  m
}

#let photo_ref = calc_photo_ref()
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
  #set page(margin: 10mm)
  #text(size: 22pt, weight: "bold")[Bildverzeichnis]
  #v(-5mm)
  #text(size: 9pt, fill: rgb("#666666"))[
    #if appendix_ref_mode == "counter" [
      Die Nummern entsprechen den kleinen Referenzzahlen auf den Fotos im Buch.
    ] else [
      2.3 = Seite 2, Photo 3, Lesereihenfolge von oben links
    ]
  ]
  #v(0mm)

  #{
    let items = ()
    let cur_group = none

    for (page_nr, page_data) in data.layout.enumerate() {
      // Seiten-Trenner
      items.push(
        block(
          width: 100%,
          above: 3mm,
          below: 1.5mm,
          fill: rgb("#dddddd"),
          inset: (x: 2mm, y: 1.5mm),
          text(size: 8pt, weight: "bold")[Seite #(page_nr + 1)],
        ),
      )

      for photo_id in page_data.photos {
        let parts = photo_id.split("/")
        let group = parts.at(0)
        let name = parts.last().split(".").at(0)
        let ref_label = photo_ref.at(photo_id, default: "")

        // Gruppen-Header bei Wechsel
        if group != cur_group {
          cur_group = group
          items.push(
            block(
              width: 100%,
              above: 2mm,
              below: 2mm,
              inset: 0mm,
              text(size: 8pt, weight: "bold", fill: rgb("#333333"))[#group],
            ),
          )
        }

        // Foto-Eintrag
        let ts = photo_ts.at(photo_id, default: none)
        items.push(
          block(
            width: 100%,
            above: 0pt,
            below: 1mm,
            text(size: 7pt)[
              #box(
                width: 6mm,
                inset: (x: 1mm, y: 0.0mm),
                align(left, text(fill: black, weight: "bold")[#ref_label]),
              )
              #try_strip_datetime_from(name)
              #if ts != none [
                #text(fill: rgb("#888888"))[ · #fmt_ts_de(ts)]
              ]
            ],
          ),
        )
      }
    }

    columns(5, gutter: 6mm)[
      #{ for item in items { item } }
    ]
  }
]



