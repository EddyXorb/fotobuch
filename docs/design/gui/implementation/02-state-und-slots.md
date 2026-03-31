# Phase 2: State + Slot-Interaktion

**Ziel**: DerivedState aufbauen, Slots erkennen und highlighten.

## 2.1 — DerivedState

- Struct mit Lookup-Maps (photo_by_id, placement_of_photo, unplaced_photos, etc.)
- `DerivedState::rebuild(&ProjectState)` — eine Methode, ein Codepfad
- Wird im Background-Thread gebaut, via Channel an UI übergeben

## 2.2 — Slot-Overlay-System

- Slot-Koordinaten (mm) → Screen-Koordinaten (abhängig von Zoom/Scroll)
- Hit-Test: Mausposition → welcher Slot?
- Hover: halbtransparentes blaues Rect
- Klick: Einzelselektion (grüne Umrandung)
- Ctrl+Klick: Toggle-Selektion, Shift+Klick: Range-Selektion

## 2.3 — Statusbar + Toolbar

- Statusbar: aktuelle Seite, Foto-Count, Selektion-Info
- Toolbar: Build, Release, Undo, Redo, Config-Button
