# GUI Implementierungsplan

## Phase 1: Minimal Viable GUI

**Ziel**: Seiten anzeigen, scrollen, zoomen. Proof of Concept.

1. **Feature-Gate einrichten**
   - `gui`-Feature in Cargo.toml mit eframe/egui/typst-render
   - `main.rs` Weiche: args → CLI, keine args → GUI
   - `src/gui/mod.rs` mit leerem `FotobuchApp`

2. **Rendering-Kern**
   - Typst-Dokument kompilieren (bestehende `compile_to_bytes` aufteilen)
   - `typst-render` pro Seite → Pixmap → demultiply → egui::TextureHandle
   - Alle Seiten beim Start rendern

3. **Hauptansicht**
   - Vertikales ScrollArea mit allen Seitenbildern
   - Zoom mit Ctrl+Scroll
   - Seitennummer als Label über jeder Seite

**Ergebnis**: Man sieht das Fotobuch, kann scrollen und zoomen.

## Phase 2: Slot-Interaktion

**Ziel**: Slots erkennen, highlighten, selektieren.

4. **Slot-Overlay-System**
   - Slot-Koordinaten (mm) → Screenkoordinaten umrechnen (abhängig von Zoom + Scroll)
   - Hit-Test: Mausposition → welcher Slot?
   - Hover: halbtransparentes blaues Rect zeichnen
   - Klick: grüne Umrandung, Selektion speichern

5. **Statusbar**
   - Aktuelle Seite (basierend auf Scroll-Position)
   - Selektiertes Foto: ID, Auflösung, DPI
   - Anzahl Fotos, unplatziert

## Phase 3: Drag & Drop + Background-Rendering

**Ziel**: Fotos swappen, live Rebuild.

6. **Drag & Drop innerhalb einer Seite**
   - Drag-Start bei Mausklick+Bewegung auf Slot
   - Visuelles Feedback: Ghost-Image am Cursor
   - Drop auf anderen Slot → Swap (default) oder Move (Shift)
   - Ratio-Feedback: Grün/Rot-Overlay auf Ziel-Slot

7. **Background-Rendering**
   - `std::sync::mpsc` Channel: Request/Result
   - Background-Thread: Solver → YAML → Typst → Render
   - Blur-Effekt: alte Textur mit Gauss-Blur (CPU, einmalig bei dirty-Markierung)
   - Neue Textur → swap in, Blur weg

8. **Undo/Redo**
   - Ctrl+Z / Ctrl+Y → StateManager::undo()/redo()
   - Nach Undo: alle Seiten als dirty markieren, neu rendern

## Phase 4: Panels

**Ziel**: Foto-Pool, Seiten-Navigation, Config.

9. **Seiten-Navigation (rechts)**
   - Thumbnails: niedrig aufgelöste Version der Seitentexturen
   - Klick → Scroll zur Seite
   - Drag & Drop → Seiten swappen/moven
   - Badge [A]/[M] pro Seite

10. **Foto-Pool (links)**
    - `ProjectState.photos` als Liste rendern
    - Gruppen als collapsible Headers
    - Pro Foto: Name, Mini-Thumbnail, Status (platziert/unplatziert)
    - Drag aus Pool auf Hauptansicht = Place
    - Tooltip: Metadaten (Größe, Timestamp, Gewicht)

11. **Config-Panel**
    - `serde_yaml::to_value(&config)` → rekursiv Widgets generieren
    - Mapping → CollapsingHeader
    - String → TextEdit, Number → DragValue, Bool → Checkbox
    - Änderung → `serde_yaml::from_value()` → Config zurückschreiben
    - Floating Window, toggle mit Ctrl+,

## Phase 5: Advanced Features

12. **Cross-Page Drag**
    - Foto von Hauptansicht auf Seiten-Nav-Thumbnail draggen
    - = Move/Swap auf andere Seite (Shift-Modifikator)

13. **Manual Mode**
    - PageMode-Enum in LayoutPage (YAML-Erweiterung)
    - [A|M]-Toggle pro Seite in der GUI
    - Manual-Seiten: Slots frei positionierbar, Solver überspringt sie
    - Drag auf freie Fläche = Slot repositionieren
    - Resize per Ecken-Drag (Ratio beibehalten)

14. **Hotkeys komplett**
    - Alle Hotkeys aus dem UX-Konzept implementieren
    - Ctrl+G: Popup mit Seitennummer-Eingabe
    - R: Rebuild aktuelle Seite

15. **Polish**
    - Zoom-Debouncing (Textur-Re-Render bei Zoom-Änderung)
    - Drag-Ghosts mit Semi-Transparenz
    - Smooth Scrolling
    - Kontextmenü (Rechtsklick)

## Abhängigkeiten

```
Phase 1 ─→ Phase 2 ─→ Phase 3 ─→ Phase 5
                  └──→ Phase 4 ─→ Phase 5
```

Phase 2 und 4 sind teilweise parallel machbar.
Phase 5 (Manual Mode, Cross-Page Drag) baut auf allem auf.

## Neue Dependencies

| Crate | Zweck | Feature-gated |
|-------|-------|---------------|
| `eframe` | egui Framework (OpenGL/wgpu Backend) | gui |
| `egui` | Immediate-Mode UI | gui |
| `typst-render` | Typst → Pixmap (nutzt tiny-skia) | gui |
| `image` | Pixmap-Konvertierung, Blur-Effekt | gui |

`typst` und `typst-kit` sind bereits Dependencies - kein Overhead.

## Aufwandsschätzung (grob)

- Phase 1: Grundgerüst, schnell machbar
- Phase 2: Slot-Overlay ist geometrisch einfach (mm → px Umrechnung)
- Phase 3: Hauptaufwand ist Background-Thread + Channel-Architektur
- Phase 4: Config-Auto-Widget ist der kreativste Teil
- Phase 5: Manual Mode erfordert sorgfältige State-Behandlung

## Risiken

1. **typst-render Kompatibilität**: Die Typst-Version muss mit typst-render matchen (gleiche typst-Version)
2. **Premultiplied Alpha**: typst-render gibt premultiplied Pixmaps, egui erwartet straight alpha → Konvertierung nötig
3. **Kompilierzeit**: Typst-Kompilierung für ein ganzes Buch kann Sekunden dauern → nur dirty pages rendern
4. **Speicher**: Eine Textur pro Seite bei hohem Zoom kann viel RAM brauchen → nur sichtbare Seiten in voller Auflösung, Rest als Thumbnails
