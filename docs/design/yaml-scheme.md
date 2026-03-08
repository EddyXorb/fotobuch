# YAML-Scheme

```yaml
config:
  book:
    title: "mein_buch"
    page_width_mm: 420.0
    page_height_mm: 297.0
    bleed_mm: 3.0
    # margin_mm, gap_mm, bleed_threshold_mm: defaulted
  # ga: defaulted
  # preview: defaulted

photos:
  - group: "2024-01-15_Urlaub"
    sort_key: "2024-01-15T09:23:00" # timestamp of minimal time of photo in group on first add
    files:
      - id: "2024-01-15_Urlaub/IMG_001.jpg" # <-- unique across all groups, if not: warning. id does not not necessarily have to be the groupfolder + file, but normally should be if there are no clashes with other files (if yes, use suffix counter "_1" etc.)
        source: "/home/user/Fotos/2024-01-15_Urlaub/IMG_001.jpg"
        width_px: 6000
        height_px: 4000
        area_weight: 1.0
        timestamp: "2024-01-15T09:23:00" 
        hash: 324345a4643a54v3...
      - id: "2024-01-15_Urlaub/IMG_002.jpg"
        file: "IMG_002.jpg"
        width_px: 4000
        height_px: 6000
        area_weight: 2.0
        timestamp: "2024-01-15T09:23:00" 
        hash: av465a4645234234v3...
  - group: "2024-02-20_Geburtstag"
    sort_key: "2024-02-20T14:00:00"
    files:
      - id: "2024-02-20_Geburtstag/IMG_010.jpg"
        ssource: /home/user/Fotos/2024-02-20_Geburtstag/IMG_010.jpg"
        width_px: 5000
        height_px: 3333
        area_weight: 1.0
        timestamp: "2024-01-15T09:23:00" 
        hash: a2345234244643a54v3...

layout:
  - page: 1
    photos:                                          # Benutzer-Input: welche Fotos (sortiert nach Ratio), nach id
      - "2024-01-15_Urlaub/IMG_002.jpg"              # 0.67 
      - "2024-01-15_Urlaub/IMG_001.jpg"              # 1.50
    slots:                                           # Solver-Output: Platzierung (Index-gekoppelt an photos)
      - x_mm: -3.0
        y_mm: -3.0
        width_mm: 148.5
        height_mm: 216.0
      - x_mm: 151.5
        y_mm: 10.0
        width_mm: 135.5
        height_mm: 190.0
  - page: 2
    photos:
      - "2024-02-20_Geburtstag/IMG_010.jpg"          # 1.50
    slots:
      - x_mm: 10.0
        y_mm: 10.0
        width_mm: 277.0
        height_mm: 190.0
```

**Anmerkungen zum Schema:**

- `config`: Alle Konfigurationsparameter. Nur Pflichtfelder (`book.page_width_mm`, `book.page_height_mm`, `book.bleed_mm`) werden von `new` erzeugt, alles andere ist defaulted und optional.
- `photos`: Importierte Fotos, gruppiert. `group` ist der Gruppenname, `files` enthaelt die einzelnen Fotos.
- `id`: Projekt-relativer Schluessel (Unique Key). Bestimmt den Cache-Pfad (`.fotobuch/cache/preview/<path>`).Wenn es clashes gibt mit gleichen dateinamen in unterschiedlichen source-foldern : füge suffix counter "_1" zur id hinzu
- `timestamp`: aufnahmedatum (oder dateidatum, wenn nicht bekannt) des fotos
- `source`: Absoluter Pfad zum Original. Wird fuer Preview-/Final-Resize und Cache-Rebuild benoetigt. Das Original wird nie veraendert.
- `sort_key`: Zeitstempel pro Gruppe fuer die chronologische Reihenfolge (von `add` per Heuristik (frühester timestamp aller hinzugefügten fotos nach erstem add) ermittelt, manuell aenderbar).
- `layout`: Das Buch-Layout. Pro Seite `photos` (Benutzer-Input) und `slots` (Solver-Output).
- `layout[].photos`: Welche Fotos auf der Seite, sortiert nach Ratio aufsteigend. Der `# 0.67`-Kommentar wird beim Schreiben per Post-Processing berechnet (`serde_yaml` unterstuetzt keine Kommentare).
- `layout[].slots`: Berechnete Platzierungen, Index-gekoppelt an `photos`. `slots[i]` ist der Slot fuer `photos[i]`.
- **Tausch innerhalb einer Seite:** Zwei Zeilen in `photos` tauschen, `slots` nicht anfassen. Fotos mit aehnlichem Ratio (Kommentar pruefen) sind problemlos tauschbar.
- **Tausch ueber Seitengrenzen:** Zeile aus `photos` der einen Seite in die andere verschieben (gleicher Index im Ziel). Aehnliches Ratio = kein Rebuild noetig.
- Bei Re-Optimierung einer Seite: Solver liest `layout[i].photos` -> findet Metadaten in `photos` (Top-Level) -> laesst GA laufen -> schreibt `layout[i].slots` neu. Die `photos`-Liste bleibt unangetastet.
- `layout[].page`: ein counter, der nur für den benutzer da ist und dem index +1 in der liste entsprechend sollte; wird nicht fürs rendering verwendet und dient nur als info; d.h. änderungen daran durch benutzer ändern das fotobuch nicht. Wird nach jedem build/rebuild neu gesetzt
- `hash`: wird bei jedem add für jedes photo berechnet und gesetzt. Der hash entsteht durch hashen der ersten und letzten 64 kb jeder datei mit blake3 (zur zeit)