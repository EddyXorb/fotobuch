// ╔══════════════════════════════════════════════════════════════════╗
// ║  USER SETTINGS — safe to edit                                    ║
// ╚══════════════════════════════════════════════════════════════════╝

// Show filename as a label centered on each photo (preview only)
#let show_image_captions_on_preview = false
// Show bleed (red) and margin (blue) border overlays (preview only)
#let show_borders_on_preview = true
// Show slot number and area weight centered on each photo, e.g. "3:1.5" (preview only)
#let show_slot_info_on_preview = true

// Append a photo index at the end of the document
#let appendix_show = false
#let appendix_nr_columns = 7
#let appendix_try_strip_datetimes_from_photo_name = true
#let appendix_show_page_nr_separator = false

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
#let show_slot_info_on_preview = show_slot_info_on_preview and not is_final

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

#let cover_or_none = data.config.book.at("cover", default: none)
#let has_cover = cover_or_none != none and cover_or_none.at("active", default: false)

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
    let n = if has_cover { 0 } else { 1 }
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
        pairs = pairs + ((photo_id, page_data.slots.at(i, default: none)),)
      }
      for (pos, p) in pairs.enumerate() {
        photo_ref.insert(
          p.at(0),
          str(if has_cover { pi } else { pi + 1 }) + "." + str(if has_cover { pos } else { pos + 1 }),
        )
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

// Map photo_id → area_weight, sourced from YAML
#let photo_weight = {
  let m = (:)
  for group in data.photos {
    for file in group.files {
      m.insert(file.id, file.area_weight)
    }
  }
  m
}

// ── Page rendering blocks ────────────────────────────────────────────

// Draws red bleed border and blue margin border as overlays (w_mm/h_mm = content size without bleed)
// b = bleed length value, m = margin length value
#let draw_borders(w_mm, h_mm, b, m) = [
  #place(top + left, dx: -(b / 2 + m), dy: -(b / 2 + m), rect(
    width: w_mm * 1mm + b,
    height: h_mm * 1mm + b,
    stroke: red + b,
    fill: none,
  ))
  #place(top + left, dx: -m / 2, dy: -m / 2, rect(
    width: w_mm * 1mm - m,
    height: h_mm * 1mm - m,
    stroke: (paint: blue, thickness: m),
    fill: none,
  ))
]

// Renders a slot: thin black frame, centered slot address, and optionally a photo with weight
#let render_slot(page_index, slot, slot_nr, photo_id, photo_ref, photo_weight) = [
  // Draw thin black frame around slot
  #place(top + left, dx: slot.x_mm * 1mm, dy: slot.y_mm * 1mm, rect(
    width: slot.width_mm * 1mm,
    height: slot.height_mm * 1mm,
    stroke: if show_slot_info_on_preview { (paint: black, thickness: 0.5pt) } else { none },
    fill: none,
  ))

  // Place image if photo_id is provided
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
  ]

  // Centered slot address (always shown when show_slot_info_on_preview is true)
  #if show_slot_info_on_preview [
    #place(
      top + left,
      dx: slot.x_mm * 1mm,
      dy: slot.y_mm * 1mm,
      box(
        width: slot.width_mm * 1mm,
        height: slot.height_mm * 1mm,
        align(center + horizon, text(size: 20pt, weight: "bold", fill: if photo_id == none { black } else { white })[
          #page_index:#slot_nr
          #if photo_id != none [ \(#str(calc.round(photo_weight.at(photo_id, default: 1.0), digits: 1))\)]
        ]),
      ),
    )
  ]

  // Reference badge (only if photo is present)
  #if photo_id != none and appendix_show and appendix_ref_mode == "counter" [
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
  #place(center + horizon, rotate(-30deg, text(
    size: 120pt,
    fill: rgb("#00000055"),
    weight: "bold",
  )[PREVIEW #page_number]))
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
      if appendix_show_page_nr_separator {
        items.push(block(
          width: 100%,
          above: 3mm,
          below: 1.5mm,
          fill: rgb("#dddddd"),
          inset: (x: 2mm, y: 1.5mm),
          text(size: 8pt, weight: "bold")[#appendix_label_page #(page_nr + 1)],
        ))
      }
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
            text(size: 8pt, weight: "bold", fill: rgb("#333333"))[#try_strip_datetime_from(group)],
          ))
        }
        let ts = photo_ts.at(photo_id, default: none)
        items.push(block(
          width: 100%,
          above: 0pt,
          below: 1mm,
          text(size: 7pt)[
            #box(width: 6mm, inset: (x: 1mm, y: 0mm), align(left, text(fill: black, weight: "bold")[#ref_label])) #h(
              0.5mm,
            )
           #fmt_ts_de(ts)  
            #if ts != none [#text(fill: rgb("#888888"))[ · #try_strip_datetime_from(name)]]
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

// ── Cover page ───────────────────────────────────────────────────────
#let inner_page_count = if has_cover { data.layout.len() - 1 } else { data.layout.len() }
#let cover_front_back_w = if has_cover { cover_or_none.front_back_width_mm } else { 0.0 }
#let cover_h = if has_cover { cover_or_none.height_mm } else { 0.0 }
#let cover_bleed = if has_cover { cover_or_none.bleed_mm * 1mm } else { 0mm }
#let cover_margin = if has_cover { cover_or_none.margin_mm * 1mm } else { 0mm }
#let spine_mode = if has_cover { cover_or_none.at("spine_mode", default: "auto") } else { "auto" }
#let spine_w = if has_cover {
  if spine_mode == "auto" {
    float(inner_page_count) / 10.0 * cover_or_none.spine_mm_per_10_pages
  } else {
    cover_or_none.spine_width_mm
  }
} else { 0.0 }
#let cover_total_w = if has_cover {
  if spine_mode == "auto" {
    cover_front_back_w + spine_w
  } else {
    cover_front_back_w
  }
} else { 0.0 }
#let spine_text_content = if has_cover { cover_or_none.at("spine_text", default: data.config.book.title) } else { "" }

#if has_cover [
  #[
    #set page(
      width: cover_total_w * 1mm + 2 * cover_bleed,
      height: cover_h * 1mm + 2 * cover_bleed,
      margin: cover_bleed + cover_margin,
    )
    #let cover_data = data.layout.at(0)
    #if show_borders_on_preview [
      #draw_borders(cover_total_w, cover_h, cover_bleed, cover_margin)
      // Spine area markers: two vertical green lines bounding the spine
      #place(top + left, dx: (cover_front_back_w / 2 - spine_w / 2) * 1mm, dy: -cover_bleed, rect(
        width: spine_w * 1mm,
        height: cover_h * 1mm + 2 * cover_bleed,
        stroke: (left: green + 0.5pt, right: green + 0.5pt, top: none, bottom: none),
        fill: rgb(0, 200, 0, 20),
      ))
    ]
    #for (i, slot) in cover_data.slots.enumerate() [
      #let photo_id = cover_data.photos.at(i, default: none)
      #render_slot(0, slot, i, photo_id, photo_ref, photo_weight)
    ]
    // Spine text — reads bottom-to-top; dx = half of front+back = single page width
    #place(top + left, dx: (cover_front_back_w / 2 - spine_w / 2) * 1mm, dy: 0mm, box(
      width: spine_w * 1mm,
      height: cover_h * 1mm,
      align(horizon + center, rotate(-90deg, box(
        stroke: if is_final { none } else { green },
        width: cover_h * 1mm,
        align(left, text(
          size: calc.min(20mm, spine_w * 0.9 * 1mm),
          h(0.05 * cover_h * 1mm) + spine_text_content,
        )),
      ))),
    ))
    #if not is_final [#render_preview_watermark("Cover")]
    #pagebreak()
  ]
]

// ── Inner pages ──────────────────────────────────────────────────────
#let layout_start = if has_cover { 1 } else { 0 }

#for page_index in range(layout_start, data.layout.len()) [
  #let page_data = data.layout.at(page_index)
  #if show_borders_on_preview [#draw_borders(
    data.config.book.page_width_mm,
    data.config.book.page_height_mm,
    bleed,
    margin,
  )]

  #for (i, slot) in page_data.slots.enumerate() [
    #let photo_id = page_data.photos.at(i, default: none)
    #render_slot(page_index, slot, i, photo_id, photo_ref, photo_weight)
  ]

  #let display_nr = page_index - layout_start + 1
  #if not is_final [#render_preview_watermark(display_nr)]

  #if page_index < data.layout.len() - 1 or appendix_show [#pagebreak()]
]

#if appendix_show [#render_appendix(photo_ref, photo_ts)]
