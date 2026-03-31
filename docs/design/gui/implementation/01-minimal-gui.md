# Phase 1: Minimal Viable GUI

**Ziel**: Seiten anzeigen, scrollen, zoomen. Kein UI-Freeze.

## 1.1 — Feature-Gate + Grundgerüst

- `gui`-Feature in Cargo.toml mit eframe/egui
- `main.rs` Weiche: args → CLI, keine args → GUI
- `src/gui.rs` mit `FotobuchApp` (eframe::App)
- Background-Thread mit Channel-Paar (task_tx/result_rx) von Anfang an

## 1.2 — Initiales Rendering

- Beim Start: `render_pages()` für alle Seiten im Background-Thread
- UI-Thread pollt `result_rx.try_recv()` jeden Frame
- Texturen als `Vec<Option<PageTexture>>` (None = noch nicht gerendert)

## 1.3 — Hauptansicht

- Vertikales ScrollArea mit Seitenbildern
- Zoom mit Ctrl+Scroll
- Seitennummer als Label über jeder Seite
- Platzhalter (grauer Rect) für noch nicht gerenderte Seiten

**Ergebnis**: Fotobuch sichtbar, scrollbar, zoombar. Kein UI-Freeze.
