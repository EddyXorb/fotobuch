# Phase 5: Cross-Page + Neue Seiten

**Ziel**: Vollständige Drag-Operationen über Seitengrenzen.

## 5.1 — [+]-Platzhalter zwischen Seiten

- Schmale Rects zwischen Seiten in der Hauptansicht
- Drop darauf → `commands::page::move(..., page+)` im Background
- Leere Seiten verschwinden automatisch (Lib-Logik)

## 5.2 — Cross-Page Drag

- Selektion auf Seite A → Drag auf Slot/Seite B (Hauptansicht oder Nav)
- M gehalten: Move, sonst Swap
- Background: entsprechender `page move`/`page swap` Command

## 5.3 — Hotkeys komplett

- Alle Hotkeys aus dem UX-Konzept
- Ctrl+G: Popup mit Seitennummer-Eingabe
- R: Rebuild selektierte Seite(n) im Background
