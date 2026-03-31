# CLI → GUI Mapping

## Erweiterte Interaktionen (Ergänzung zu 01-ux-konzept)

### Multi-Selektion

| Aktion | Ergebnis |
|--------|----------|
| Klick auf Slot | Einzelselektion (ersetzt vorherige) |
| Ctrl+Klick | Slot zur Selektion hinzufügen/entfernen (Toggle) |
| Shift+Klick | Range-Selektion: alle Slots zwischen letztem Klick und aktuellem |
| Escape | Selektion aufheben |

Selektion ist immer auf **eine Seite** beschränkt.

### Drag-Operationen mit Selektion

| Aktion | Ergebnis |
|--------|----------|
| Drag Selektion → Slot auf anderer Seite | **Swap** (S gedrückt) oder **Move** (M gedrückt) |
| Drag Selektion → [+]-Platzhalter zwischen Seiten | **Move auf neue Seite** (= `page move 3:2 3+`) |
| Drag Selektion → Foto-Pool (links) | **Unplace** |
| Drag Selektion → Seiten-Thumbnail (rechts) | Move/Swap auf diese Seite |

### Modifier-Hotkeys beim Drag

| Taste | Modus | Statusbar-Anzeige |
|-------|-------|-------------------|
| (kein) | Swap | `[Drag: Swap]` |
| `M` halten | Move | `[Drag: Move]` |

### Neue-Seite-Platzhalter

Zwischen jeder Seite in der Hauptansicht: schmales Rechteck mit `[+]`, halbtransparent.
- Hover: leuchtet auf
- Drop darauf: erstellt neue Seite an dieser Position, verschiebt Slots dorthin
- Kein Drop: unsichtbar/minimal (stört nicht beim Betrachten)
- Leere Seiten nach Move/Unplace werden automatisch entfernt (wie CLI)

### Seiten-Drag in der Navigation (rechts)

| Aktion | Ergebnis |
|--------|----------|
| Drag Thumbnail hoch/runter | Seite verschieben (Reihenfolge ändern) |
| Drag Thumbnail auf anderes Thumbnail | Seiten swappen |

---

## CLI-Kommandos → GUI-Äquivalente

| CLI-Kommando | Beispiel | GUI-Pendant |
|---|---|---|
| **`init`** | `fotobuch init mein-buch --width 420 --height 297 --bleed 3` | Dialog beim ersten Start: Projektname, Maße eingeben. Oder: File → New Project |
| **`project new`** | `fotobuch project new mein-buch --width 420 --height 297` | Identisch zu init: Neues-Projekt-Dialog |
| **`project list`** | `fotobuch project list` | Dropdown im Toolbar: Projektliste |
| **`project switch`** | `fotobuch project switch anderes-buch` | Dropdown-Auswahl im Toolbar |
| **`add`** | `fotobuch add ./fotos -r` | Drag & Drop von Ordnern/Dateien auf das Fenster. Oder: Toolbar-Button / Ctrl+O |
| **`add --filter`** | `fotobuch add ./fotos --filter "2024-06"` | Add-Dialog mit optionalem Regex-Filter |
| **`add --weight 2.0`** | `fotobuch add ./fotos --weight 2.0` | Add-Dialog mit Gewicht-Eingabe |
| **`build`** | `fotobuch build` | Toolbar-Button [Build] / Ctrl+B. Passiert auch automatisch nach jeder Änderung |
| **`build --pages 3,5`** | `fotobuch build --pages 3,5` | Rechtsklick auf Seite → "Rebuild Page". Oder: `R`-Taste auf selektierter Seite |
| **`build release`** | `fotobuch build release` | Toolbar-Button [Release] / Ctrl+Shift+B |
| **`rebuild --page 3`** | `fotobuch rebuild --page 3` | `R`-Taste bei selektierter Seite, oder Rechtsklick → Rebuild |
| **`rebuild --range-start 2 --range-end 5 --flex 1`** | (wie links) | Multi-Selektion in Seiten-Nav (Shift+Klick), dann Rechtsklick → "Rebuild Range". Flex als Option im Kontextmenü |
| **`rebuild --all`** | `fotobuch rebuild --all` | Toolbar: [Build] lang drücken oder Rechtsklick → "Rebuild All" |
| **`place`** | `fotobuch place` | Drag aus Foto-Pool auf eine Seite. Oder: Toolbar → "Place All" |
| **`place --into 5`** | `fotobuch place --into 5` | Drag aus Foto-Pool direkt auf Seite 5 |
| **`place --filter "urlaub"`** | `fotobuch place --filter urlaub` | Foto-Pool hat Suchfeld oben, dann "Place filtered" Button |
| **`unplace 3:2`** | `fotobuch unplace 3:2` | Slot selektieren → Delete-Taste. Oder: Drag auf Foto-Pool (links) |
| **`unplace 3:2..5,7`** | `fotobuch unplace 3:2..5,7` | Multi-Selektion (Shift+Klick Slot 2, Shift+Klick Slot 5, Ctrl+Klick Slot 7) → Delete |
| **`page swap 3 5`** | `fotobuch page swap 3 5` | In Seiten-Nav: Thumbnail von Seite 3 auf Seite 5 draggen |
| **`page swap 3:2 5:6`** | `fotobuch page swap 3:2 5:6` | Slot 3:2 selektieren → auf Slot 5:6 draggen (Default=Swap) |
| **`page swap 3:2..4 5:6..8`** | `fotobuch page swap 3:2..4 5:6..8` | Multi-Selektion 3:2..4 → auf Slot 5:6 draggen (Swap-Modus) |
| **`page move 3:2 to 5`** | `fotobuch page move 3:2 to 5` | Slot 3:2 selektieren → auf Seite-5-Thumbnail draggen (M halten) |
| **`page move 3:2 to 3+`** | `fotobuch page move 3:2 to 3+` | Slot 3:2 auf [+]-Platzhalter nach Seite 3 draggen |
| **`page move 3 to 5`** | `fotobuch page move 3 to 5` | In Seiten-Nav: Seite 3 auf Position 5 draggen |
| **`page move 3:2 out`** | `fotobuch page move 3:2 out` | = Unplace: Slot selektieren → Delete oder Drag auf Foto-Pool |
| **`page split 3:4`** | `fotobuch page split 3:4` | Shift+Klick auf Slot 4 bis letzten Slot → auf [+]-Platzhalter nach Seite 3 draggen |
| **`page combine 3,5`** | `fotobuch page combine 3,5` | In Seiten-Nav: alle Slots von Seite 5 auf Seite 3 draggen (M halten). Leere Seite 5 verschwindet automatisch |
| **`page info 3`** | `fotobuch page info 3` | Hover über Slots zeigt Tooltips. Rechtsklick → Page Info für Details |
| **`page info 3:2`** | `fotobuch page info 3:2` | Klick auf Slot → Statusbar zeigt Details (ID, Pixel, DPI, Gewicht) |
| **`page weight 3:2 2.0`** | `fotobuch page weight 3:2 2.0` | Rechtsklick auf Slot → "Set Weight" → DragValue |
| **`remove "urlaub/*"`** | `fotobuch remove "urlaub/*"` | Im Foto-Pool: Rechtsklick auf Gruppe → "Remove Group" |
| **`remove --unplaced`** | `fotobuch remove --unplaced` | Im Foto-Pool: Button "Remove Unplaced" oder Kontextmenü |
| **`status`** | `fotobuch status` | Statusbar zeigt permanent: Seiten, Fotos, Unplatziert, Build-Status |
| **`status 3`** | `fotobuch status 3` | Klick auf Seite → Details in Statusbar oder Tooltip |
| **`config`** | `fotobuch config` | Ctrl+, → Config-Panel (Floating Window) |
| **`history -n 10`** | `fotobuch history -n 10` | Undo-Button lang drücken → History-Liste als Dropdown |
| **`undo`** | `fotobuch undo` | Ctrl+Z |
| **`undo 3`** | `fotobuch undo 3` | Ctrl+Z dreimal, oder History-Dropdown → Eintrag auswählen |
| **`redo`** | `fotobuch redo` | Ctrl+Y |

## Aktualisierte Hotkey-Tabelle

| Taste | Aktion |
|-------|--------|
| `Ctrl+Z` / `Ctrl+Y` | Undo / Redo |
| `Ctrl+B` | Build (inkrementell) |
| `Ctrl+Shift+B` | Release Build |
| `Ctrl+Scroll` | Zoom |
| `Ctrl+0` | Zoom: Seitenbreite einpassen |
| `Ctrl+G` | Gehe zu Seite |
| `Home` / `End` | Erste / Letzte Seite |
| `Delete` | Selektierte Slots unplacen |
| `R` | Selektierte Seite(n) rebuild |
| `Ctrl+,` | Config-Panel toggle |
| `Ctrl+O` | Fotos hinzufügen (Add-Dialog) |
| `Escape` | Selektion aufheben |
| `Ctrl+A` | Alle Slots der aktuellen Seite selektieren |
| `M` (halten) | Während Drag: Move statt Swap |
| `Shift+Klick` | Range-Selektion |
| `Ctrl+Klick` | Toggle-Selektion |
