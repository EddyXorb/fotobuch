// ╔══════════════════════════════════════════════════════════════════╗
// ║  USER SETTINGS — safe to edit                                    ║
// ╚══════════════════════════════════════════════════════════════════╝

// Show filename as a label centered on each photo (preview only)
#let show_image_captions_on_preview = true
// Show bleed (red) and margin (blue) border overlays (preview only)
#let show_borders_on_preview = true

// Append a photo index at the end of the document
#let appendix_show = true
#let appendix_nr_columns = 5
#let appendix_try_strip_datetimes_from_photo_name = true

// Reference mode for the photo index:
// "counter"   = sequential number shown as a badge on each photo
// "positions" = no badge; reference = "page.position" (e.g. "2.3")
#let appendix_ref_mode = "positions"

// Labels used in the appendix (localise as needed)
#let appendix_label_title = "Bildverzeichnis"
#let appendix_label_page = "Seite"

// ════════════════════════════════════════════════════════════════════
//  TEMPLATE — edit below only if you know Typst well
// ════════════════════════════════════════════════════════════════════

// Final mode: true = print output, no watermarks or guide lines
//             false = preview with watermark, bleed and margin overlays
#let is_final = false

// Override preview flags when rendering the final version
#let show_image_captions_on_preview = show_image_captions_on_preview and not is_final
#let show_borders_on_preview = show_borders_on_preview and not is_final

#let project_name = "{project_name}"
#let data = yaml(project_name + ".yaml")
#set text(font: "Libertinus Serif")

// Image path prefix depending on output mode
#let cache_prefix = if is_final {
  ".fotobuch/cache/" + project_name + "/final/"
} else {
  ".fotobuch/cache/" + project_name + "/preview/"
}

// Page size and margins from YAML (including bleed)
#let bleed = data.config.book.bleed_mm * 1mm
#let margin = data.config.book.margin_mm * 1mm
#set page(
  width: data.config.book.page_width_mm * 1mm + 2 * bleed,
  height: data.config.book.page_height_mm * 1mm + 2 * bleed,
  margin: bleed + margin,
)

// ── Utility functions ────────────────────────────────────────────────

// Strips an ISO date prefix (e.g. "2024-01-15T120000_") from a filename
#let try_strip_datetime_from(s) = {
  if appendix_try_strip_datetimes_from_photo_name {
    s.replace(regex("^\d{4}-\d{2}-\d{2}[@T]\d{6}[_-]?"), "")
  } else {
    s
  }
}

// Formats an ISO-8601 timestamp to "1. Jan 2024 10:10 Uhr"
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

// Builds a map photo_id → reference label (counter: "1","2",… / positions: "2.3")
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

// Map photo_id → capture timestamp, sourced from YAML
#let photo_ts = {
  let m = (:)
  for group in data.photos {
    for file in group.files {
      m.insert(file.id, file.timestamp)
    }
  }
  m
}

// ── Page rendering blocks ────────────────────────────────────────────

// Draws red bleed border and blue margin border as overlays
#let draw_borders() = [
  #place(top + left, dx: -(bleed / 2 + margin), dy: -(bleed / 2 + margin), rect(
    width: data.config.book.page_width_mm * 1mm + bleed,
    height: data.config.book.page_height_mm * 1mm + bleed,
    stroke: red + bleed,
    fill: none,
  ))
  #place(top + left, dx: -margin / 2, dy: -margin / 2, rect(
    width: data.config.book.page_width_mm * 1mm - margin,
    height: data.config.book.page_height_mm * 1mm - margin,
    stroke: (paint: blue, thickness: margin),
    fill: none,
  ))
]

// Places a single photo in its slot — with optional filename label and reference badge
#let render_photo(slot, photo_id, photo_ref) = [
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
  #if appendix_show and appendix_ref_mode == "counter" [
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

// Renders the PREVIEW watermark diagonally across the page
#let render_preview_watermark(page_number) = [
  #place(center + horizon, rotate(-30deg, text(size: 120pt, fill: rgb("#00000055"), weight: "bold")[PREVIEW #page_number]) )
]

// Renders the photo index: page separators, group headers, entries with timestamps
#let render_appendix(photo_ref, photo_ts) = [
  #set page(margin: 10mm)
  #text(size: 22pt, weight: "bold")[#appendix_label_title]
  #v(-5mm)
  #text(size: 9pt, fill: rgb("#666666"))[
    #if appendix_ref_mode == "counter" [
    ] else [
      x.y = #appendix_label_page x, Photo y
    ]
  ]
  #v(0mm)
  #{
    let items = ()
    let cur_group = none
    for (page_nr, page_data) in data.layout.enumerate() {
      items.push(block(
        width: 100%,
        above: 3mm,
        below: 1.5mm,
        fill: rgb("#dddddd"),
        inset: (x: 2mm, y: 1.5mm),
        text(size: 8pt, weight: "bold")[#appendix_label_page #(page_nr + 1)],
      ))
      for photo_id in page_data.photos {
        let parts = photo_id.split("/")
        let group = parts.at(0)
        let name = parts.last().split(".").at(0)
        let ref_label = photo_ref.at(photo_id, default: "")
        if group != cur_group {
          cur_group = group
          items.push(block(
            width: 100%,
            above: 2mm,
            below: 2mm,
            inset: 0mm,
            text(size: 8pt, weight: "bold", fill: rgb("#333333"))[#group],
          ))
        }
        let ts = photo_ts.at(photo_id, default: none)
        items.push(block(
          width: 100%,
          above: 0pt,
          below: 1mm,
          text(size: 7pt)[
            #box(width: 6mm, inset: (x: 1mm, y: 0mm), align(left, text(fill: black, weight: "bold")[#ref_label]))
            #try_strip_datetime_from(name)
            #if ts != none [#text(fill: rgb("#888888"))[ · #fmt_ts_de(ts)]]
          ],
        ))
      }
    }
    columns(appendix_nr_columns, gutter: 4mm)[#{ for item in items { item } }]
  }
]

// ════════════════════════════════════════════════════════════════════
//  MAIN — render pages
// ════════════════════════════════════════════════════════════════════

#let photo_ref = calc_photo_ref()

#for (page_index, page_data) in data.layout.enumerate() [
  #if show_borders_on_preview [#draw_borders()]

  #for (i, slot) in page_data.slots.enumerate() [
    #let photo_id = page_data.photos.at(i, default: none)
    #if photo_id != none [#render_photo(slot, photo_id, photo_ref)]
  ]

  #if not is_final [#render_preview_watermark(page_index + 1)]

  #if page_index < data.layout.len() - 1 or appendix_show [#pagebreak()]
]

#if appendix_show [#render_appendix(photo_ref, photo_ts)]
