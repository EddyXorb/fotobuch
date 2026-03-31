# Phase 6: Manual Mode + Polish

## 6.1 — Manual Mode

- [A|M]-Toggle pro Seite
- Manual-Seiten: Drag auf freie Fläche = Slot repositionieren
- Resize per Ecken-Drag (Ratio beibehalten)
- Solver überspringt Manual-Seiten

## 6.2 — Polish

- Zoom-Debouncing (Re-Render ~200ms nach letzter Zoom-Änderung)
- Render-Cancellation bei neuer Anfrage
- Nur sichtbare Seiten in voller Auflösung, Rest als Thumbnails
- Drag-Ghosts, Smooth Scrolling, Kontextmenü
