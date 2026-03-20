# YAML-Scheme

```yaml
config:
  book:
    title: "mein_buch"
    page_width_mm: 420.0
    page_height_mm: 297.0
    bleed_mm: 3.0
    # margin_mm, gap_mm, bleed_threshold_mm: defaulted
    cover:                          # optional — fehlt dieser Block, gibt es kein Cover
      active: true                  # false = Cover-Block vorhanden aber deaktiviert
      spine_mm_per_10_pages: 1.4   # Rückendicke pro 10 Seiten
      front_back_width_mm: 594.0   # Gesamtbreite Vorder- + Rückseite, ohne Buchrücken
      height_mm: 297.0             # Höhe des Covers
      spine_text: ~                # optional, default = book.title
      bleed_mm: 3.0
      margin_mm: 0.0
      gap_mm: 5.0
      bleed_threshold_mm: 3.0
  # ga: defaulted
  # preview: defaulted
  #   cover_separate_pdf: false    # TODO: Cover als eigenes PDF ausgeben (noch nicht implementiert)

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
  - page: 0                                            # Cover: immer erster Eintrag wenn cover.active = true
    photos:
      - "2024-01-15_Urlaub/IMG_001.jpg"              # 1.50
    slots:
      - x_mm: -3.0
        y_mm: -3.0
        width_mm: 843.0
        height_mm: 303.0
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

## Designentscheidungen: Seitenindizierung und Cover-Handling

### Seitenindizierung: `layout[].page` = Array-Index (0-basiert)

`layout[].page` ist immer gleich der Position im `layout`-Array (0-basiert). Es gibt keine
1-basierten Seitenzahlen im Code mehr. Das gilt mit und ohne Cover:

- `layout[0].page = 0` (Cover, falls aktiv; sonst erste Innenseite)
- `layout[1].page = 1` (erste oder zweite Innenseite)
- `layout[N].page = N`

**Konsequenz:** Keinerlei Umrechnung zwischen `page_nr` und Array-Index nötig.
Für alle internen Funktionen (`rebuild_single_page`, `collect_photos_as_groups`, `BuildResult.pages_rebuilt`, usw.) sind 0-basierte Array-Indizes der einzige Seitenreferenz.

**Anzeige für den Nutzer:** Seiten werden ebenfalls 0-basiert angezeigt (`--page 0` = Cover / erste Seite).

### Cover-Seite: ausschließlich SinglePage-Solver

Die Cover-Seite (`layout[0]` bei aktivem Cover) darf **ausschließlich** vom SinglePage-Solver
berechnet werden. Kein Mixing mit normalen Innenseiten in MultiPage-Builds.

**Verhalten bei `build` (inkrementell):**
- Wenn der Cover-Eintrag (`layout[0]`) als veraltet erkannt wird, wird er **nicht neu berechnet**.
- Stattdessen erscheint eine Warnung: `"Cover page changed — use rebuild --page 0 to rebuild it explicitly."`
- Exception: Beim allerersten Build (`layout` ist leer) wird das Cover wie jede andere Seite
  im MultiPage-Solver verteilt und dann per SinglePage gelayoutet.

**Verhalten bei `rebuild`:**
- `rebuild --page 0`: SinglePage-Solver für das Cover (normal).
- `rebuild --range 0-N` (enthält Cover): Cover wird zuerst mit dem SinglePage-Solver gelöst,
  die restlichen Seiten `1..=N` gehen wie gewohnt an den MultiPage-Solver.
- `rebuild` (alle Seiten): Cover wird per SinglePage gelöst; alle Innenseiten gehen an
  den MultiPage-Solver. Die Foto-Zuweisung des Covers bleibt unverändert (nur Slots werden
  neu berechnet); alle übrigen Fotos werden neu verteilt.

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
- `layout[].page`: Immer gleich dem Array-Index (0-basiert). Wird nach jedem build/rebuild neu gesetzt: `page = index`. Mit und ohne Cover gilt die gleiche Regel. Der Nutzer referenziert Seiten mit `--page 0`, `--page 1` usw. Keine Umrechnung nötig.
- `hash`: wird bei jedem add für jedes photo berechnet und gesetzt. Der hash entsteht durch hashen der ersten und letzten 64 kb jeder datei mit blake3 (zur zeit)

## Cover

### Aktivierung

Cover ist aktiv wenn `config.book.cover` vorhanden **und** `cover.active = true`. `fotobuch project new` legt keinen Cover-Block an. Nachträgliches Hinzufügen:

```
fotobuch add cover --spine_mm_per_10_pages <N> [--width_mm <W>] [--height_mm <H>]
```

Dieser Befehl setzt `config.book.cover` mit `active: true` in der YAML. Der erste `layout`-Eintrag wird automatisch zur Coverseite — kein Flag auf dem `layout`-Eintrag nötig.

`active: false` erlaubt es, Cover-Konfiguration temporär zu deaktivieren ohne den Block zu löschen.

### Cover-Seite im Layout

Wenn `cover.active = true`: der **erste** `layout`-Eintrag ist das Cover. Kein explizites Flag auf dem Eintrag. Das Template und `renumber_pages` leiten die Cover-Existenz allein aus `config.book.cover.active` ab.

### Dimensionen

`cover.front_back_width_mm` ist die Breite von Vorder- und Rückseite zusammen, ohne Buchrücken. `cover.height_mm` ist die Höhe des Covers. Beide Felder sind Pflicht wenn `cover` vorhanden ist. Die Gesamtbreite im Template ergibt sich als `front_back_width_mm + spine_width_mm`.

### Bunddicke

```
spine_width_mm = (inner_page_count / 10.0) * spine_mm_per_10_pages
```

`inner_page_count` = `layout.len() - 1` (alle Einträge außer dem ersten, wenn Cover aktiv). Wer Doppelseiten verwendet, gibt die Bunddicke pro 10 Doppelseiten an; wer Einzelseiten nutzt, halbiert den Wert entsprechend.

### Buchrücken-Text

- Default: `book.title`
- Überschreibbar mit `cover.spine_text`
- Schriftgröße: automatisch, max. `spine_width_mm * 0.8`
- Position: 5 % vom unteren Rand des Rückens, Leserichtung nach oben (90° CCW rotiert)
- Rendering erfolgt im Template, nicht im Solver

### Solver-Integration

Der Solver darf eine Cover-Seite **nur als Einzelseite** berechnen (`page_layout_solver` mit expliziten Cover-Dimensionen). Keine Mischung mit normalen Innenseiten in Multi-Page-Builds.

### Template-Verhalten

1. Template prüft `data.config.book.cover` und `cover.active`
2. Wenn aktiv: erster `layout`-Eintrag wird mit Cover-Dimensionen gerendert (als eigene Seite vor allen anderen)
3. Danach folgen `layout[1..]` als normale Innenseiten
4. Im Preview wird das Cover als erste Seite angezeigt
5. Bleed gilt auch für das Cover, keine Margins

### Geplant (noch nicht implementiert)

- `config.preview.cover_separate_pdf: true` — gibt das Cover als eigenes PDF aus, getrennt vom Innenteil
