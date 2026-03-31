# Phase 4: Panels

**Ziel**: Foto-Pool, Seiten-Navigation, Config.

## 4.1 — Seiten-Navigation (rechts)

- Thumbnails: niedrig aufgelöste Seitentexturen
- Klick → Scroll zur Seite
- Drag → Seiten swappen/moven
- Badge [A]/[M] pro Seite
- Drag-Target für Cross-Page Slot-Operationen

## 4.2 — Foto-Pool (links)

- Gruppen als collapsible Headers, Fotos als Liste
- Pro Foto: Name, Mini-Thumbnail (Background geladen), platziert/unplatziert
- Drag aus Pool auf Seite/Nav → `commands::place()`
- Drag von Slot auf Pool → `commands::unplace()`
- Thumbnails im Background laden

## 4.3 — Config-Panel

- `serde_yaml::to_value(&config)` → rekursiv Widgets generieren
- Mapping → CollapsingHeader, String → TextEdit, f64 → DragValue, bool → Checkbox
- Änderung → `serde_yaml::from_value()` → Config-Command im Background
- Floating Window, toggle mit Ctrl+,
