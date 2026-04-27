# GUI UX-Konzept

## Grundidee

Die GUI ist ein **interaktiver Typst-Viewer**. Die gerenderten Seiten SIND das Ergebnis - kein separates Preview. Die GUI legt nur einen dünnen Interaktionslayer darüber.

## Layout: Drei-Panel-Architektur

```
┌──────────────────────────────────────────────────────────────┐
│  fotobuch · my-project     [Rebuild] [Release] [↩][↪] [⚙]  │
├───────────┬──────────────────────────────────┬───────────────┤
│ FOTO-POOL │       HAUPTANSICHT               │ SEITEN-NAV   │
│           │                                  │              │
│ ▼ Urlaub  │   ┌──────────────────┐           │ ┌──┐ P0 [A] │
│  img1.jpg │   │  Seite 0         │  [A|M]    │ │  │        │
│  img2.jpg │   └──────────────────┘           │ └──┘        │
│  img3.jpg │                                  │ ┌──┐ P1 [A] │
│           │   ┌──────────────────┐           │ │  │        │
│ ▶ Familie │   │  Seite 1         │  [A|M]    │ └──┘        │
│           │   │ ┌──┐ ┌────────┐  │           │ ┌──┐ P2 [M] │
│ ▶ Wandern │   │ │  │ │ hover! │  │           │ │  │        │
│           │   │ └──┘ └────────┘  │           │ └──┘        │
│           │   └──────────────────┘           │ ...         │
│           │                                  │              │
│           │   ... (vertical scroll) ...      │              │
├───────────┴──────────────────────────────────┴───────────────┤
│  Seite 3/24 · 156 Fotos · 12 unplatziert · [Shift=Move]     │
└──────────────────────────────────────────────────────────────┘
```

### Linkes Panel: Foto-Pool

- Spiegelt 1:1 die `photos`-Sektion des YAML
- Gruppen als aufklappbare Ordner
- Pro Foto: Mini-Thumbnail + Name, Tooltip mit Metadaten
- Platzierte Fotos gedimmt mit Seitennummer-Badge
- Unplatzierte Fotos visuell hervorgehoben
- Drag aus Pool auf Seite = Place
- Startversion: einfache Liste/Tabelle, später evtl. Grid-View

### Mitte: Hauptansicht

- Alle Seiten vertikal gestapelt, scrollbar
- Jede Seite = Typst-gerendertes Raster-Image als egui-Textur
- Zoom: Ctrl+Scroll, stufenlos
- Pro Seite ein kleiner [A|M]-Toggle rechts von der Seite (Auto/Manual Mode)
- Seitenheader mit Seitennummer

### Rechtes Panel: Seiten-Navigation

- Thumbnail pro Seite (klein, ~80px breit)
- Klick = Scroll zur Seite
- Drag & Drop = Seiten swappen/moven
- [A]/[M]-Badge pro Seite sichtbar
- Auch Drag-Target für Foto-Operationen (Cross-Page Move/Swap)

## Interaktionsmodell

### Selektion

| Aktion | Ergebnis |
|--------|----------|
| Klick auf Slot | Einzelselektion (ersetzt vorherige) |
| Ctrl+Klick | Slot zur Selektion hinzufügen/entfernen |
| Shift+Klick | Range-Selektion (Slot X bis Slot Y, wie Textauswahl) |
| Ctrl+A | Alle Slots der aktuellen Seite |
| Escape | Selektion aufheben |

Selektion ist immer auf **eine Seite** beschränkt.

### Slot-Interaktion (Hauptansicht)

| Aktion | Ergebnis |
|--------|----------|
| Hover über Slot | Halbtransparentes blaues Overlay (alpha ~0.15), Tooltip mit Foto-Info |
| Drag Selektion → anderer Slot | **Swap** (default) oder **Move** (M halten) |
| Drag Selektion → [+]-Platzhalter zwischen Seiten | **Move auf neue Seite** |
| Drag Selektion → Foto-Pool (links) | **Unplace** |
| Drag Selektion → Seiten-Thumbnail (rechts) | Cross-Page Move/Swap |
| Drag Slot → freie Fläche (Manual Mode) | Freie Positionierung, Slot bekommt neue x/y |
| Drag Slot-Ecke (Manual Mode) | Resize unter Beibehaltung des Seitenverhältnisses |
| Rechtsklick Slot | Kontextmenü: Unplace, Rebuild, Weight, Info |
| Delete-Taste | Selektierte Slots unplacen |

### Neue-Seite-Platzhalter

Zwischen jeder Seite: schmales Rechteck mit `[+]`, halbtransparent.
- Normalzustand: minimal/unauffällig
- Hover: leuchtet auf
- Drop darauf: erstellt neue Seite, verschiebt Slots dorthin
- Leere Seiten nach Move/Unplace verschwinden automatisch (wie CLI)

### Drag-Feedback

- **Grünes Overlay** auf Ziel-Slot: Gleiche Aspect Ratio → problemloser Swap
- **Rötliches Overlay** auf Ziel-Slot: Unterschiedliche Ratio → Solver muss Seite neu layouten
- **Statusbar** zeigt während Drag: `[Drag: Swap]` bzw. `[M: Move]`

### Auto-Rebuild nach Änderungen — Architekturprinzip

**Grundregel**: Jede Aktion, die das Layout mutiert, triggert automatisch
einen Rebuild der betroffenen Seiten. Es gibt **keinen** manuellen
inkrementellen Build-Trigger (Hotkey/Button) — Anzeige und State sind
immer konsistent ohne expliziten User-Befehl.

1. Swap/Move/Unplace/Place/PageSwap/PagePos/Config-Änderung → die Lib
   liefert `pages_modified ∪ pages_inserted`; die GUI markiert diese
   sofort als „dirty".
2. Background-Thread baut dirty Pages neu (Solver + Typst-Render).
3. Während Rebuild: **Gaussian Blur + kleiner Spinner** über der Seite.
4. Alte Version bleibt sichtbar (geblurt) bis neue fertig.
5. UI bleibt immer bei 60 fps.

**Manuelle Trigger** (alle anderen Build-/Rebuild-Aktionen sind
explizit):

| Trigger | Wirkung |
|---------|---------|
| `R` auf selektierter Seite | `rebuild --page N` (Solver-Rerun einzelner Seiten) |
| Toolbar-Button `[Rebuild]` mit Selektion | identisch zu `R` |
| Toolbar-Button `[Rebuild]` ohne Selektion | öffnet Confirm-Dialog mit Begründung „keine Seite ausgewählt" → bei Bestätigung `rebuild --all` |
| `Ctrl+Shift+B` / Toolbar-Button `[Release]` | `build release` (PDF/Print-Output) |

### Seiten-Modus: Auto vs Manual

- **Auto** (default, implizit): Solver optimiert Layout bei Rebuild
- **Manual**: Solver lässt Seite in Ruhe, User positioniert frei
- Toggle pro Seite: kleiner Schalter [A|M] neben der Seite
- YAML: optionales `mode`-Feld pro `LayoutPage` (fehlt = auto)
- Wechsel Manual→Auto: Solver optimiert Seite beim nächsten Build neu

## Hotkeys

| Taste | Aktion |
|-------|--------|
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+Shift+B` | Release Build (PDF/Print-Output) |
| `Ctrl+Scroll` | Zoom |
| `Ctrl+0` | Zoom: Seitenbreite einpassen |
| `Ctrl+G` | Gehe zu Seite |
| `Home` / `End` | Erste / Letzte Seite |
| `Delete` | Selektierten Slot unplacen |
| `R` | Selektierte Seite rebuild |
| `Ctrl+,` | Config-Panel toggle |
| `Escape` | Selektion aufheben |
| `M` (halten) | Drag-Modus: Move statt Swap |
| `Ctrl+Klick` | Toggle-Selektion (einzelne Slots) |
| `Shift+Klick` | Range-Selektion (Slot X bis Y) |
| `Ctrl+A` | Alle Slots der aktuellen Seite selektieren |
| `Ctrl+O` | Fotos hinzufügen (Add-Dialog) |

## Config-Panel

Öffnet sich als **Floating Window** (Ctrl+,):

- Config-Structs werden **automatisch** über `serde_yaml::Value` zu Widgets
- Rekursive Regel:
  - `Mapping` → Collapsible Section
  - `String` → TextEdit
  - `Number` → DragValue
  - `Bool` → Checkbox
  - Enum (String-Variante) → ComboBox
- Feld-Labels = YAML-Keys, human-readable formatiert (snake_case → Title Case)
- Änderungen → sofort in Config übernehmen → dirty pages → Auto-Rebuild
- **Null Wartungsaufwand** bei Config-Änderungen in der Lib
