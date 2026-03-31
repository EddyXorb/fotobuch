# GUI Implementierungsplan

## Phase 0: Lib-Vorbereitung

**Ziel**: Lib-Änderungen die GUI ermöglichen, ohne GUI-Code zu schreiben.

1. **`CommandOutput<T>` einführen**
   - `StateManager::finish()` → `Result<ProjectState>` (move statt drop)
   - Alle Commands returnen `Result<CommandOutput<T>>` mit ihrem bestehenden Result-Typ + State
   - CLI-Handler: nur `.result` nutzen (minimale Anpassung)

2. **`render_pages()` in der Lib**
   - `output::typst::render_pages(root, name, pages, pixel_per_pt) → Vec<RenderedPage>`
   - Intern: `typst::compile()` → `typst_render::render()` → premultiply→straight Konvertierung
   - `typst-render` als neue Dependency (nicht feature-gated, klein)

3. **`PageMode` in LayoutPage**
   - `mode: PageMode` (Default: Auto, optional in YAML)
   - Solver überspringt Manual-Seiten bei inkrementellem Build

## Phase 1: Minimal Viable GUI

**Ziel**: Seiten anzeigen, scrollen, zoomen. Proof of Concept.

4. **Feature-Gate + Grundgerüst**
   - `gui`-Feature in Cargo.toml mit eframe/egui
   - `main.rs` Weiche: args → CLI, keine args → GUI
   - `src/gui.rs` mit `FotobuchApp` (eframe::App)
   - Background-Thread mit Channel-Paar (task_tx/result_rx) von Anfang an

5. **Initiales Rendering**
   - Beim Start: `render_pages()` für alle Seiten im Background-Thread
   - UI-Thread pollt `result_rx.try_recv()` jeden Frame
   - Texturen als `Vec<Option<PageTexture>>` (None = noch nicht gerendert)

6. **Hauptansicht**
   - Vertikales ScrollArea mit Seitenbildern
   - Zoom mit Ctrl+Scroll
   - Seitennummer als Label über jeder Seite
   - Platzhalter (grauer Rect) für noch nicht gerenderte Seiten

**Ergebnis**: Fotobuch sichtbar, scrollbar, zoombar. Kein UI-Freeze.

## Phase 2: State + Slot-Interaktion

**Ziel**: `DerivedState` aufbauen, Slots erkennen und highlighten.

7. **DerivedState**
   - Struct mit Lookup-Maps (photo_by_id, placement_of_photo, unplaced_photos, etc.)
   - `DerivedState::rebuild(&ProjectState)` — eine Methode, ein Codepfad
   - Wird im Background-Thread gebaut, via Channel an UI übergeben

8. **Slot-Overlay-System**
   - Slot-Koordinaten (mm) → Screen-Koordinaten (abhängig von Zoom/Scroll)
   - Hit-Test: Mausposition → welcher Slot?
   - Hover: halbtransparentes blaues Rect
   - Klick: Einzelselektion (grüne Umrandung)
   - Ctrl+Klick: Toggle-Selektion, Shift+Klick: Range-Selektion

9. **Statusbar + Toolbar**
   - Statusbar: aktuelle Seite, Foto-Count, Selektion-Info
   - Toolbar: Build, Release, Undo, Redo, Config-Button

## Phase 3: Commands + Background-Pipeline

**Ziel**: GUI-Aktionen führen Lib-Commands aus, alles non-blocking.

10. **Command-Dispatch**
    - User-Aktion → `task_tx.send(RunCommand(...))` → Background führt aus
    - Background sendet `CommandDone` → UI updatet `project_state` + `derived`
    - Background rendert dirty pages → UI swappt Texturen

11. **Swap/Move (gleiche Seite)**
    - Drag-Start auf Slot → DragState
    - Drop auf anderen Slot → Background: `commands::page::swap()`
    - Ratio-Feedback: grün (gleiche Ratio) / rot (unterschiedlich)
    - M-Taste gehalten: Move statt Swap

12. **Blur-Effekt + Undo/Redo**
    - Dirty page → Blur über alter Textur + Spinner
    - Neue Textur fertig → Blur entfernen
    - Ctrl+Z/Y → Background: `commands::undo()`/`redo()` → alle Seiten dirty

## Phase 4: Panels

**Ziel**: Foto-Pool, Seiten-Navigation, Config.

13. **Seiten-Navigation (rechts)**
    - Thumbnails: niedrig aufgelöste Seitentexturen
    - Klick → Scroll zur Seite
    - Drag → Seiten swappen/moven
    - Badge [A]/[M] pro Seite
    - Drag-Target für Cross-Page Slot-Operationen

14. **Foto-Pool (links)**
    - Gruppen als collapsible Headers, Fotos als Liste
    - Pro Foto: Name, Mini-Thumbnail (Background geladen), platziert/unplatziert
    - Drag aus Pool auf Seite/Nav → `commands::place()`
    - Drag von Slot auf Pool → `commands::unplace()`
    - Thumbnails im Background laden

15. **Config-Panel**
    - `serde_yaml::to_value(&config)` → rekursiv Widgets generieren
    - Mapping → CollapsingHeader, String → TextEdit, f64 → DragValue, bool → Checkbox
    - Änderung → `serde_yaml::from_value()` → Config-Command im Background
    - Floating Window, toggle mit Ctrl+,

## Phase 5: Cross-Page + Neue Seiten

**Ziel**: Vollständige Drag-Operationen über Seitengrenzen.

16. **[+]-Platzhalter zwischen Seiten**
    - Schmale Rects zwischen Seiten in der Hauptansicht
    - Drop darauf → `commands::page::move(..., page+)` im Background
    - Leere Seiten verschwinden automatisch (Lib-Logik)

17. **Cross-Page Drag**
    - Selektion auf Seite A → Drag auf Slot/Seite B (Hauptansicht oder Nav)
    - M gehalten: Move, sonst Swap
    - Background: entsprechender `page move`/`page swap` Command

18. **Hotkeys komplett**
    - Alle Hotkeys aus dem UX-Konzept
    - Ctrl+G: Popup mit Seitennummer-Eingabe
    - R: Rebuild selektierte Seite(n) im Background

## Phase 6: Manual Mode + Polish

19. **Manual Mode**
    - [A|M]-Toggle pro Seite
    - Manual-Seiten: Drag auf freie Fläche = Slot repositionieren
    - Resize per Ecken-Drag (Ratio beibehalten)
    - Solver überspringt Manual-Seiten

20. **Polish**
    - Zoom-Debouncing (Re-Render ~200ms nach letzter Zoom-Änderung)
    - Render-Cancellation bei neuer Anfrage
    - Nur sichtbare Seiten in voller Auflösung, Rest als Thumbnails
    - Drag-Ghosts, Smooth Scrolling, Kontextmenü

## Abhängigkeiten

```
Phase 0 ─→ Phase 1 ─→ Phase 2 ─→ Phase 3 ─→ Phase 5 ─→ Phase 6
                                      └──→ Phase 4 ─→ Phase 5
```

Phase 0 ist reine Lib-Arbeit, kein GUI-Code.
Phase 3 und 4 sind teilweise parallel machbar.

## Neue Dependencies

| Crate | Zweck | Feature-gated |
|-------|-------|---------------|
| `eframe` | egui Framework (OpenGL/wgpu Backend) | gui |
| `egui` | Immediate-Mode UI | gui |
| `typst-render` | Typst → Pixmap (nutzt tiny-skia) | nein (Lib) |
| `image` | Blur-Effekt | gui |

`typst` und `typst-kit` sind bereits Dependencies.

## Risiken

1. **typst-render Version**: Muss zur verwendeten typst-Version passen
2. **Premultiplied Alpha**: `render_pages()` konvertiert intern — einmal korrekt implementieren, dann erledigt
3. **Typst-Kompilierzeit**: Ganzes Dokument wird kompiliert auch wenn nur eine Seite gerendert wird — bei großen Büchern ggf. Cache für `typst::Document` im Background-Thread
4. **RAM bei Zoom**: Hochaufgelöste Texturen nur für sichtbare Seiten, Rest als Thumbnails halten
