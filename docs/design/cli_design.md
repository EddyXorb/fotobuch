# fotobuch CLI — Design-Dokument

Stand: 2026-03-07

## Leitprinzipien

- **Nutzerperspektive zuerst.** Die CLI-Struktur ergibt sich aus den Aktionen des Benutzers, nicht aus der internen Architektur.
- **Wenige, selbsterklaerende Kommandos.** Vorbilder: `cargo`, `uv`, modernes `git`.
- **Schnelles Feedback.** Jede Aktion gibt sofort Rueckmeldung. Langsame Operationen (Layout-Berechnung, Bildexport) sind explizite eigene Kommandos.
- **Textbasiert und editierbar.** Der Projektzustand liegt in YAML, das Layout in Typst. Beides menschenlesbar, versionierbar, manuell anpassbar.
- **Ein Projekt pro Verzeichnis** (git-Modell). Kein `--project`-Flag noetig — der cwd ist der Kontext.

---

## 1  `fotobuch new`

### Was der Benutzer erwartet

Er will ein neues Fotobuch-Projekt starten. Er hat Fotos auf der Platte verstreut und will sie zu einem Buch zusammenstellen.

### Warum `new` statt `init`

`init` (git-Modell) verknuepft das Projekt mit dem aktuellen Verzeichnis — einmalig, nicht wiederholbar. Das ist verwirrend wenn man ein zweites Buch aus denselben Fotos machen will.

`new` (cargo-Modell) erstellt ein eigenes Verzeichnis. Wiederholbar, eigenstaendig. Zwei Buecher aus denselben Fotos = zwei Verzeichnisse.

```
fotobuch new --width 420 --height 297 mein-buch
fotobuch new --width 420 --height 297 highlights
```

Multi-Projekt ist damit organisch geloest, ohne `--project`-Flag.

### Interface

```
$ fotobuch new --help
Create a new photobook project

Usage: fotobuch new [OPTIONS] --width <MM> --height <MM> <n>

Arguments:
  <n>  Project name (becomes the directory name)

Options:
      --width <MM>   Page width in mm
      --height <MM>  Page height in mm
      --bleed <MM>   Bleed margin in mm [default: 3]
  -h, --help         Print help
```

Breite und Hoehe sind bewusst Pflichtangaben — es gibt keinen sinnvollen Default, der fuer alle Druckanbieter passt. Wer bei Saal Digital A3 quer bestellt, muss die exakten Masse aus deren Spezifikation uebernehmen.

Perspektivisch koennte ein `--template saal-a3-quer`-Flag hinzukommen, das bekannte Formate als Presets anbietet. Fuer den Anfang reicht die explizite Angabe.

### Verhalten

1. Erstellt Verzeichnis `<n>/`.
2. Erstellt `fotobuch.yaml` darin mit den Seitenmassen (noch keine Fotos/Seiten).
3. Erstellt `.fotobuch/cache/` mit Preview- und Final-Unterordnern.
4. `git init --initial-branch=fotobuch` + `.gitignore` (ignoriert `.fotobuch/`, PDFs).
5. Initialer Commit mit `fotobuch.yaml`.
6. Ausgabe: `Created project "mein-buch" in ./mein-buch/ (420x297mm, 3mm bleed)`

**Beispiel:**

```
$ fotobuch new --width 420 --height 297 mein-buch
Created project "mein-buch" in ./mein-buch/ (420x297mm, 3mm bleed)
```

### Resultierende Struktur

```
mein-buch/
├── .git/
├── .gitignore
├── fotobuch.yaml
└── .fotobuch/
    └── cache/
        ├── preview/
        └── final/
```

`.gitignore` wird von `fotobuch new` erzeugt:

```gitignore
.fotobuch/
*.pdf
```

Getrackt werden `fotobuch.yaml` und die `.typ`-Templates. Cache und PDFs sind ableitbar und gehoeren nicht ins Repo.

Die YAML-, Typst- und PDF-Dateien liegen im Projekt-Root (nicht im versteckten Ordner), weil Typst nur relative Pfade in Unterverzeichnisse aufloesen kann und weil die Dateien fuer den Benutzer direkt sichtbar und editierbar sein sollen.

### Git-Versionierung

`fotobuch new` fuehrt `git init --initial-branch=fotobuch` aus. Der Branch-Name sorgt dafuer, dass Shell-Prompts (bash, zsh) neben dem Verzeichnisnamen `fotobuch` anzeigen — ein sofortiger Hinweis, dass es sich um ein Fotobuch-Projekt handelt. Jeder Solver-Aufruf (`build`, `rebuild`) erzeugt automatisch Commits:

- **Pre-Commit** vor dem Solver-Lauf: erfasst manuelle Aenderungen des Benutzers seit dem letzten Build.
- **Post-Commit** nach dem Solver-Lauf: erfasst das Solver-Ergebnis.

Commit-Messages sind strukturiert:

| Anlass | Format |
|---|---|
| Projekt erstellt | `new: 420x297mm, 3mm bleed` |
| Fotos hinzugefuegt | `add: 47 photos in 3 groups` |
| Pre-Build | `pre-build: pages 2,3 modified` |
| Post-Build | `post-build: 12 pages (cost: 0.0842)` |
| Pre-Rebuild | `pre-rebuild: pages 2,5` |
| Post-Rebuild | `post-rebuild: pages 2,5 (cost: 0.0312)` |

`git diff HEAD~1` zeigt dann exakt was der Solver veraendert hat. `git diff HEAD~2 HEAD~1` zeigt was der Benutzer manuell geaendert hat. `git log --oneline` gibt die vollstaendige Projekthistorie.

Umsetzung via `std::process::Command` — die Operationen sind trivial. Falls `git` nicht im PATH ist: Warnung loggen, History ueberspringen. Kein harter Fehler.

---

## 2  `fotobuch add`

### Was der Benutzer erwartet

Er will Fotos aus verschiedenen Quellen zum Projekt hinzufuegen. Nicht alles auf einmal — er kann `add` mehrfach aufrufen.

### Interface

```
$ fotobuch add --help
Add photos to the project

Usage: fotobuch add [OPTIONS] <PATH>...

Arguments:
  <PATH>...  Directories or individual files to add

Options:
      --allow-duplicates  Allow adding files with identical content
  -h, --help              Print help
```

### Verhalten

#### Gruppierung

Jedes Verzeichnis, das direkt Bilddateien enthaelt, wird eine eigene Gruppe. Der Gruppenname ist der relative Pfad ab dem `add`-Argument.

```
fotobuch add ~/Fotos/Urlaub/

~/Fotos/Urlaub/
├── Tag1/
│   ├── IMG_001.jpg
│   └── IMG_002.jpg
├── Tag2/
│   └── IMG_003.jpg
├── Abend/
│   └── Kneipe/
│       └── IMG_010.jpg
└── panorama.jpg
```

Ergibt vier Gruppen:

| Gruppe | Fotos |
|---|---|
| `Urlaub` | `panorama.jpg` |
| `Urlaub/Tag1` | `IMG_001.jpg`, `IMG_002.jpg` |
| `Urlaub/Tag2` | `IMG_003.jpg` |
| `Urlaub/Abend/Kneipe` | `IMG_010.jpg` |

Reine Durchgangsverzeichnisse (wie `Abend/`, das selbst keine Bilder enthaelt) werden uebersprungen — keine leeren Gruppen.

#### Einzeldateien

```
fotobuch add ~/Fotos/portrait.jpg
```

Gruppe wird aus dem Elternverzeichnis abgeleitet — Gruppe heisst `Fotos`. Mehrere Einzeldateien aus demselben Ordner landen in derselben Gruppe.

#### Gruppenreihenfolge

Die Reihenfolge der Gruppen im Buch ist chronologisch. Der Zeitstempel einer Gruppe wird per Heuristik bestimmt (erste verfuegbare Quelle gewinnt):

1. Ordnername parsen (z.B. `2024-01-15_Urlaub` -> `2024-01-15`)
2. Fruehestes EXIF-Aufnahmedatum (`DateTimeOriginal`) der enthaltenen Fotos
3. Frueheste File-mtime der enthaltenen Fotos

Der ermittelte Zeitstempel wird als `sort_key` pro Gruppe im YAML persistiert. Der Benutzer kann ihn dort manuell aendern, falls die Heuristik falsch lag.

Kein `--position`-Flag. Sonderfaelle (Titelseite, Nachwort) loest der Benutzer durch Editieren des `sort_key` im YAML.

#### Was bei `add` passiert (und was nicht)

Bei `add` passiert:
- Bilddateien im Verzeichnisbaum finden
- EXIF-Daten lesen (Timestamp, Dimensionen)
- Gruppen anlegen, Zeitstempel bestimmen, Fotos einsortieren
- YAML aktualisieren
- Git-Commit: `add: 47 photos in 3 groups`
- Ausgabe: `Added 47 photos in group "Urlaub/Tag1" (2024-01-15)`

Bei `add` passiert *nicht*:
- Preview-Rendering (erst bei `build`)
- Layout-Berechnung (erst bei `build`)

#### Duplikaterkennung

Erkennung ueber partiellen Hash: erste 64 KB + letzte 64 KB + Dateigroesse. Schnell genug fuer tausende Dateien, zuverlaessig genug fuer Foto-Duplikate.

**Selber Ordner nochmal ge-`add`et:**
Warnung ausgeben, nur neue Dateien hinzufuegen. Bereits bekannte Dateien (selber absoluter Pfad) werden uebersprungen. Veraenderte Originale werden erst beim `build` neu eingelesen — `add` merkt sich den Pfad und die initiale Metadaten-Erfassung.

**Selber Dateiinhalt aus anderem Ordner (Hash-Kollision):**
Warnung: `IMG_001.jpg (aus ~/Backup/) hat identischen Inhalt wie Urlaub/IMG_001.jpg — Duplikat?`
Datei wird *nicht* hinzugefuegt, ausser `--allow-duplicates` ist gesetzt. Das schuetzt vor dem haeufigsten Fehler (Backup-Ordner versehentlich doppelt hinzugefuegt), laesst sich aber ueberschreiben.

### Ausgabe-Beispiele

```
$ fotobuch add ~/Fotos/2024-01-15_Urlaub/
  Added group "2024-01-15_Urlaub" (47 photos, 2024-01-15)

$ fotobuch add ~/Fotos/2024-02-20_Geburtstag/ ~/Fotos/2024-03-01_Wanderung/
  Added group "2024-02-20_Geburtstag" (23 photos, 2024-02-20)
  Added group "2024-03-01_Wanderung" (15 photos, 2024-03-01)

$ fotobuch add ~/Fotos/2024-01-15_Urlaub/
  Warning: group "2024-01-15_Urlaub" already exists
  Skipped 47 known files, added 3 new files

$ fotobuch add ~/Backup/Urlaub/
  Warning: 45 files have identical content to existing photos (use --allow-duplicates to add anyway)
  Added 2 new photos to group "Urlaub"
```

---

## 3  `fotobuch remove`

### Was der Benutzer erwartet

Symmetrische Operation zu `add` — Fotos oder ganze Gruppen aus dem Projekt entfernen. Ohne CLI muesste der Benutzer das Foto aus `photos[].files[]` *und* aus `layout[].photos` entfernen *und* den zugehoerigen Eintrag in `layout[].slots` loeschen, damit die Index-Kopplung stimmt. Das ist fehleranfaellig.

### Interface

```
$ fotobuch remove --help
Remove photos or groups from the project

Usage: fotobuch remove [OPTIONS] <PATTERN>...

Arguments:
  <PATTERN>...  Photo paths, group names, or glob patterns

Options:
      --keep-files  Only remove from layout, keep in photos (makes them unplaced)
  -h, --help        Print help
```

### Verhalten

**Default:** Entfernt aus `photos` (Top-Level) *und* aus `layout`. Slots werden automatisch angepasst (Eintraege entfernen, Indizes nachruecken). Betroffene Seiten werden beim naechsten `build` neu gelayoutet.

**`--keep-files`:** Entfernt nur aus `layout`, behaelt das Foto in `photos`. Das Foto wird "unplaced" — das Gegenstueck zu `place`.

### Beispiele

```
fotobuch remove "2024-01-15_Urlaub/IMG_001.jpg"           # einzelnes Foto
fotobuch remove "2024-01-15_Urlaub"                        # ganze Gruppe
fotobuch remove "2024-01-15_Urlaub/IMG_00*.jpg"            # Glob-Pattern
fotobuch remove --keep-files "2024-01-15_Urlaub/IMG_005.jpg"  # nur aus Layout
```

### Symmetrie der Kommandos

```
add      <->  remove              (Projekt-Ebene: photos)
place    <->  remove --keep-files (Layout-Ebene: layout[].photos)
```

### Ausgabe

```
$ fotobuch remove "2024-01-15_Urlaub/IMG_001.jpg"
  Removed from photos: 2024-01-15_Urlaub/IMG_001.jpg
  Removed from layout: page 3, slot 2
  Page 3: needs rebuild (5 photos remaining)

$ fotobuch remove "2024-01-15_Urlaub"
  Removed group "2024-01-15_Urlaub" (47 photos)
  Removed from layout: pages 1-4 (47 slots)
  Pages 1-4: need rebuild

$ fotobuch remove --keep-files "2024-01-15_Urlaub/IMG_005.jpg"
  Removed from layout: page 2, slot 4 (photo kept in project)
  Page 2: needs rebuild (6 photos remaining)
```

---

## 4  `fotobuch status`

### Was der Benutzer erwartet

Er will wissen, wo sein Projekt steht. Was ist drin, was hat sich geaendert, was muss neu gerechnet werden.

### Interface

```
$ fotobuch status --help
Show project status

Usage: fotobuch status [PAGE]

Arguments:
  [PAGE]  Show detailed info for a specific page

Options:
  -h, --help  Print help
```

### Projektzustaende

| Zustand | Bedeutung |
|---|---|
| `empty` | Fotos hinzugefuegt, noch nie gebaut |
| `clean` | Layout existiert, nichts veraendert seit letztem Build |
| `modified` | Layout existiert, YAML wurde seit letztem Build veraendert |

### Aenderungserkennung

`status` laedt die YAML aus dem letzten Git-Commit (`git show HEAD:fotobuch.yaml`) und die aktuelle Datei, deserialisiert beide nach `ProjectState`, und vergleicht strukturell:

```rust
let committed: ProjectState = serde_yaml::from_slice(&git_show_output)?;
let current: ProjectState = project::load("fotobuch.yaml")?;

let modified_pages: Vec<usize> = current.layout.iter()
    .zip(committed.layout.iter())
    .enumerate()
    .filter(|(_, (a, b))| a.photos != b.photos || a.slots != b.slots)
    .map(|(i, _)| i + 1)
    .collect();
```

Kein Parsing von Git-Diff-Output. Git ist nur der Storage, der Vergleich passiert auf Struct-Ebene.

### Kompakte Ansicht: `fotobuch status`

```
$ fotobuch status
85 photos in 6 groups (5 unplaced)

Layout: 12 pages, 7.1 photos/page avg
  4 pages modified since last build
    pages 2, 5: need rebuild (ratio mismatch in swapped photos)
    pages 3, 8: compatible swaps only (no rebuild needed)
```

Moegliche Zustaende in der Kompaktansicht:

**Noch nie gebaut:**
```
$ fotobuch status
85 photos in 6 groups

No layout yet. Run `fotobuch build` to generate.
```

**Alles sauber:**
```
$ fotobuch status
85 photos in 6 groups

Layout: 12 pages, 7.1 photos/page avg (clean)
```

**Mit Problemen:**
```
$ fotobuch status
85 photos in 6 groups (5 unplaced)

Layout: 12 pages, 7.1 photos/page avg
  3 pages modified since last build
    pages 2, 5: need rebuild (photos moved between pages)
    page 8: swaps only (no rebuild needed)
  Warning: 2 photos in pages have no entry in groups (orphaned)
    page 3: IMG_099.jpg
    page 7: IMG_155.jpg
```

### Detail-Ansicht: `fotobuch status <page>`

```
$ fotobuch status 5
Page 5: 6 photos, modified since last build

  Slot  Photo                              Ratio  Swap group
  1     2024-01-15_Urlaub/IMG_002.jpg      0.67   A
  2     2024-01-15_Urlaub/IMG_012.jpg      0.75   B
  3     2024-02-20_Geb/IMG_022.jpg         0.75   B
  4     2024-01-15_Urlaub/IMG_001.jpg      1.50   C
  5     2024-01-15_Urlaub/IMG_004.jpg      1.50   C
  6     2024-01-15_Urlaub/IMG_007.jpg      1.78   D

  Swaps within a group (B, C) need no rebuild.
```

Ratio ist immer `width / height` (Landscape > 1, Portrait < 1). Swap-Gruppen werden on-the-fly berechnet (Ratio-Toleranz ~5%), nicht im YAML gespeichert. Fotos mit kompatiblem Seitenverhaeltnis koennen getauscht werden ohne das Layout neu zu berechnen — die Platzierung bleibt identisch, nur der Bildinhalt aendert sich.

Die Slots sind in der Status-Ansicht nach Ratio sortiert, analog zur Reihenfolge in `photos` im YAML.

### Konsistenzpruefungen

`status` prueft und meldet:

**Unplaced photos** (in `photos`, nicht in `layout`): Info, kein Fehler. Fotos die bewusst oder versehentlich keiner Seite zugeordnet sind. Werden nicht automatisch eingefuegt.

**Orphaned placements** (in `layout`, nicht in `photos`): Warnung. Die Metadaten (Dimensionen, Aspect Ratio) fehlen. PDF-Export funktioniert trotzdem (die Platzierungsmasse reichen), aber Resize fuer Final-Export kann fehlschlagen weil der `source`-Pfad fehlt. `status` listet die betroffenen Seiten und Fotos.

**Aspect-Ratio-Mismatch nach Tausch**: Wenn ein Foto in einen Slot getauscht wurde, dessen Verhaeltnis nicht passt (ueber ~5% Abweichung), kommt eine Warnung mit Seitenangabe. Das Bild wird dann im PDF verzerrt oder beschnitten dargestellt.

### Verhalten

- `status` veraendert nichts — rein lesend.
- `status` funktioniert immer, auch ohne Git (dann ohne Aenderungserkennung).
- Die Kompaktansicht ist auf wenige Zeilen begrenzt — kein Scrollen.
- Die Detail-Ansicht zeigt alles fuer eine Seite, aber nicht mehr.

---

## 5  Preview-Annotationen

Das Preview-PDF kann optional Hilfsinformationen anzeigen, die das Arbeiten mit dem YAML erleichtern:

- **Dateinamen + Ratio** unter jedem Foto (z.B. `2024-01-15_Urlaub/IMG_001.jpg (1.50)`)
- **Seitenzahlen** am Seitenrand

Gesteuert ueber `fotobuch.yaml`:

```yaml
config:
  preview:
    annotations: true    # Dateinamen unter Fotos
    page_numbers: true   # Seitenzahlen
```

Default: beides `true`. Das finale PDF (`fotobuch export`) ignoriert diese Einstellungen — dort erscheinen weder Annotationen noch Seitenzahlen (es sei denn, der Benutzer erweitert das Final-Template manuell).

Die Umsetzung liegt komplett im Typst-Template. Das Template liest die Flags via `#yaml()` und rendert die Annotationen per `#place()` + `#text()` unter/neben den Fotos. Keine Aenderung am Solver noetig.

---

## 6  `fotobuch build`

### Was der Benutzer erwartet

Wie `cargo build` — inkrementell und sicher. Beim ersten Aufruf wird alles berechnet. Danach nur das, was sich geaendert hat. Man kann `build` jederzeit ausfuehren ohne etwas kaputt zu machen.

### Interface

```
$ fotobuch build --help
Calculate layout and generate preview PDF

Usage: fotobuch build [OPTIONS]

Options:
      --pages <N,M,...>  Only consider these pages (default: all)
      --release          Build final PDF at full resolution (300 DPI, no watermarks)
  -h, --help             Print help
```

### Verhalten

**Erster Aufruf** (kein `layout`-Element im YAML):
1. Preview-Cache erzeugen (fehlende/veraltete Bilder herunterrechnen + Wasserzeichen).
2. Book-Layout-Solver: Alle Fotos aus `photos` auf Seiten verteilen (schreibt `layout[].photos`). Implizites `place` — passiert immer wenn `layout` fehlt oder leer ist.
3. Page-Layout-Solver (GA): Fuer jede Seite `run_ga()` aufrufen (schreibt `layout[].slots`).
4. YAML schreiben, Typst kompilieren, Git-Commit.

Falls der Benutzer das `layout`-Element im YAML manuell loescht, verhalt sich der naechste `build` wie ein erster Aufruf — alle Fotos werden von Grund auf verteilt. Das ist ein bewusster Reset-Mechanismus.

**Folgeaufrufe** (Layout vorhanden):
1. Preview-Cache pruefen, fehlende/veraltete Previews nacherzeugen.
2. Pro Seite pruefen ob ein Rebuild noetig ist (Details siehe Aenderungserkennung).
3. Page-Layout-Solver nur fuer Seiten die es brauchen.
4. Nichts geaendert -> `Nothing to do.`

`build` ruft den Book-Layout-Solver (Verteilung) **nur beim ersten Mal** auf. Danach nie wieder — die Zuweisung von Fotos zu Seiten ist ab dann Sache des Benutzers (via YAML) oder von `rebuild`.

### Aenderungserkennung

`build` vergleicht pro Seite den aktuellen YAML-Stand mit dem letzten Commit (analog zu `status`). Eine Seite braucht einen Rebuild wenn:

- Fotos hinzugefuegt oder entfernt wurden (Laenge von `photos` hat sich geaendert)
- Ein Foto durch ein anderes mit **anderem Ratio** ersetzt wurde (auch seitenuebergreifend)
- `area_weight` eines Fotos in `photos` geaendert wurde

Eine Seite braucht **keinen** Rebuild wenn:

- Fotos mit gleichem Ratio getauscht wurden (innerhalb der Seite oder seitenuebergreifend) — die Geometrie der Slots bleibt identisch, nur der Bildinhalt aendert sich
- Nur Metadaten in `photos` geaendert wurden die das Layout nicht beeinflussen (z.B. `sort_key`)
- Nichts geaendert wurde

### Ausgabe

**Erster Build:**
```
$ fotobuch build
  Building preview cache... 85/85 images
  Distributing 85 photos across pages... 12 pages
  Optimizing layouts...
    Page  1: 7 photos (cost: 0.0421)
    Page  2: 8 photos (cost: 0.0387)
    ...
    Page 12: 6 photos (cost: 0.0512)
  Compiling preview PDF... done
  Wrote fotobuch_preview.pdf (12 pages)
```

**Inkrementell:**
```
$ fotobuch build
  Preview cache up to date.
  2 pages need rebuild (5, 8)
  Optimizing layouts...
    Page  5: 6 photos (cost: 0.0398)
    Page  8: 7 photos (cost: 0.0445)
  Compiling preview PDF... done
  Wrote fotobuch_preview.pdf (12 pages)
```

**Nichts zu tun:**
```
$ fotobuch build
  Nothing to do. Layout is up to date.
```

**Nur Swaps:**
```
$ fotobuch build
  Preview cache up to date.
  Pages 3, 7: compatible swaps only, no rebuild needed.
  Compiling preview PDF... done
  Wrote fotobuch_preview.pdf (12 pages)
```

Auch bei reinen Swaps wird das Preview-PDF neu kompiliert — die Bilder in den Slots haben sich ja geaendert, auch wenn die Geometrie gleich bleibt.

---

## 7  `fotobuch place`

### Was der Benutzer erwartet

Nach dem Hinzufuegen neuer Fotos (via `add`) sollen diese ins bestehende Layout eingepflegt werden, ohne das gesamte Layout neu zu berechnen.

### Interface

```
$ fotobuch place --help
Place unplaced photos into the book

Usage: fotobuch place [OPTIONS]

Arguments:
  (none)

Options:
      --filter <PATTERN>  Only place photos matching this pattern
      --into <PAGE>       Place onto a specific page
  -h, --help              Print help
```

### Verhalten

`place` sortiert unplaced Fotos (in `photos`, nicht in `layout`) chronologisch in die `photos`-Listen der passenden Seiten ein. Kein Balancing, keine Layout-Berechnung — nur Zuweisung.

```
fotobuch place                       # alle unplaced, chronologisch einsortieren
fotobuch place --filter "Urlaub"     # nur matchende Gruppen/Pfade
fotobuch place --into 5              # alle unplaced auf Seite 5
```

Die chronologische Einsortierung basiert auf dem Timestamp des Fotos relativ zu den Timestamps der bereits platzierten Fotos. Wenn Seite 3 Fotos vom 15.01. und Seite 4 Fotos vom 20.01. hat, landet ein Foto vom 17.01. auf Seite 3.

Das kann dazu fuehren, dass eine Seite viele Fotos hat. Das ist Absicht — `status` zeigt es an, `rebuild` korrigiert es.

### Wann braucht man `place`?

Selten. Nur wenn nach dem ersten `build` neue Fotos dazukommen. Im Standardworkflow (alle Fotos adden, einmal bauen, dann tweaken) kommt `place` nicht zum Einsatz.

---

## 8  `fotobuch rebuild`

### Was der Benutzer erwartet

Explizites Neuberechnen — maechtiger als `build`, erfordert Absicht. Fuer den Fall, dass man ein Layout erzwingen oder Seiten neu verteilen will.

### Interface

```
$ fotobuch rebuild --help
Force re-optimization of pages or page ranges

Usage: fotobuch rebuild [OPTIONS] [PAGE_OR_RANGE]

Arguments:
  [PAGE_OR_RANGE]  Page number (e.g. 5) or range (e.g. 3-7)

Options:
      --flex <N>   Allow page count to vary by +/- N in a range [default: 0]
  -h, --help       Print help
```

### Verhalten je nach Argument

**Einzelne Seite** (`fotobuch rebuild 5`):
Page-Layout-Solver auf Seite 5, erzwungen (auch wenn `build` sie als "clean" einstufen wuerde). Foto-Zuweisung bleibt, nur `layout` wird neu geschrieben.

**Seitenbereich** (`fotobuch rebuild 3-7`):
Book-Layout-Solver auf die Teilmenge: Fotos aus Seiten 3-7 werden auf 3-7 neu verteilt (`photos`-Listen neu geschrieben), dann Page-Layout-Solver fuer jede dieser Seiten. Umliegende Seiten bleiben unangetastet. Seitenzahl bleibt gleich (5 Seiten rein, 5 Seiten raus).

Mit `--flex 2`: Solver darf den Bereich auf 3-9 Seiten strecken oder auf 3-5 schrumpfen. Nuetzlich wenn nach `place` zu viele Fotos auf wenigen Seiten gelandet sind.

**Ohne Argument** (`fotobuch rebuild`):
Alles von vorn — alle Fotos aus `groups` (inklusive bisher unplaced), Book-Layout-Solver + Page-Layout-Solver fuer alle Seiten. Wie ein erster `build`. Manuelle Aenderungen gehen verloren (aber via Git wiederherstellbar).

### Photos-Quelle

| Kommando | Photos-Quelle | Solver |
|---|---|---|
| `rebuild` | Alle aus `photos` (Top-Level) | Book-Layout + Page-Layout |
| `rebuild 3-7` | Nur aus `layout[3..7].photos` | Book-Layout (Teilmenge) + Page-Layout |
| `rebuild 5` | Nur aus `layout[5].photos` | Page-Layout |

### Abgrenzung zu `build`

| | `build` | `rebuild` |
|---|---|---|
| Verteilungssolver | Nur beim allerersten Aufruf | Bei Range oder ohne Argument |
| Page-Layout-Solver | Nur fuer geaenderte Seiten | Erzwungen fuer angegebene Seiten |
| Sicher (nichts kaputt) | Ja | Nur bei Einzelseite |
| Inkrementell | Ja | Nein |

### `--flex`

Default `--flex 0` — sicher, keine Ueberraschungen bei der Seitenzahl. Der Solver (MIP) bekommt `pages_min` und `pages_max` als Constraint:

```
fotobuch rebuild 3-7               # pages_min=5, pages_max=5
fotobuch rebuild 3-7 --flex 2      # pages_min=3, pages_max=7
```

`--flex` ist nur bei Ranges relevant. Bei Einzelseiten und bei `rebuild` ohne Argument wird es ignoriert (Einzelseite hat fixe Fotozahl, voller Rebuild bestimmt die Seitenzahl frei).

### Ablauf

1. **Pre-Commit:** `pre-rebuild: page 5` / `pre-rebuild: pages 3-7` / `pre-rebuild: all`
2. Preview-Cache pruefen.
3. Solver(s) ausfuehren.
4. YAML schreiben.
5. Typst kompilieren.
6. **Post-Commit:** `post-rebuild: page 5 (cost: 0.0312)`

---

## 9  `fotobuch build --release`

### Was der Benutzer erwartet

Wie `cargo build --release` — das finale Ergebnis, druckfertig. Wird aufgerufen wenn das Layout steht und man das PDF an Saal Digital schicken will.

### Verhalten

1. **Pruefen:** Layout muss existieren und `clean` sein (keine ungebauten Aenderungen). Falls `modified` -> Fehler mit Hinweis auf `fotobuch build` oder `fotobuch rebuild`.
2. **Final-Cache erzeugen:** Fuer jedes Foto im Layout:
   - Ziel-Pixelgroesse aus Slot berechnen: `px = layout[i].width_mm / 25.4 * 300` (analog fuer Hoehe).
   - Original von `source`-Pfad lesen.
   - Downsampling (Lanczos3 bei Faktor <= 2, Triangle bei > 2). Kein Upsampling — falls Original kleiner als Zielgroesse, Original verwenden und Warnung loggen.
   - JPEG Qualitaet 95, ablegen unter `.fotobuch/cache/final/<path>`.
   - **Immer aus dem Original.** Kein inkrementelles Resampling, keine Wiederverwendung alter Final-Bilder. Sicherheit geht vor Geschwindigkeit.
3. **Typst kompilieren:** `fotobuch_final.typ` -> Final-PDF. Kein Wasserzeichen, keine Annotationen, keine Seitenzahlen.
4. **Git-Commit:** `release: 12 pages, 85 photos`
5. **Validierung:** Nach dem Kompilieren pruefen ob alle eingebetteten Bilder die Ziel-DPI erreichen. Warnungen fuer Fotos unter 300 DPI ausgeben mit genauer Angabe (z.B. `IMG_042.jpg: 247 DPI on page 5 (original too small: 3000x2000 px for 154x103 mm slot)`).

### Ausgabe

```
$ fotobuch build --release
  Layout is clean.
  Building final images from originals... 80/85
    Warning: IMG_042.jpg only 247 DPI on page 5 (original 3000x2000 px)
    Warning: IMG_071.jpg only 289 DPI on page 9 (original 4200x2800 px)
  Compiling final PDF... done
  Wrote fotobuch_final.pdf (12 pages, 85 photos)
  2 images below 300 DPI (see warnings above)
```

### Unterschied Preview vs. Release

| | Preview (`build`) | Release (`build --release`) |
|---|---|---|
| Bilder | Cache/Preview (1200 px, Wasserzeichen) | Cache/Final (300 DPI aus Original) |
| Resampling | Inkrementell (mtime-Vergleich) | Immer voll aus Original |
| Annotationen | Dateiname + Ratio unter Fotos | Keine |
| Seitenzahlen | Ja (konfigurierbar) | Nein |
| Typst-Template | `fotobuch_preview.typ` | `fotobuch_final.typ` |
| Wann | Oft, waehrend der Arbeit | Einmal am Ende |

---

## 10  `fotobuch history`

### Was der Benutzer erwartet

Schnelle Uebersicht: was ist wann passiert im Projekt.

### Interface

```
$ fotobuch history --help
Show project change history

Usage: fotobuch history

Options:
  -h, --help  Print help
```

### Verhalten

Wrapper um `git log --oneline --format="%ai %s"` im Projektverzeichnis. Zeigt Datum + Commit-Message, ohne Hash:

```
$ fotobuch history
  2024-03-07 14:22  release: 12 pages, 85 photos
  2024-03-07 14:15  post-build: 12 pages (cost: 0.0842)
  2024-03-07 13:50  pre-build: pages 5, 8 modified
  2024-03-07 13:50  post-build: 12 pages (cost: 0.0891)
  2024-03-06 20:12  add: 15 photos in 1 group
  2024-03-06 19:45  post-build: 12 pages (cost: 0.0923)
  2024-03-06 19:44  add: 70 photos in 5 groups
  2024-03-06 19:43  new: 420x297mm, 3mm bleed
```

Fuer detailliertere Analyse: Der Benutzer kennt git und kann direkt `git log`, `git diff`, `git checkout` etc. verwenden.

---

## 11  `fotobuch config`

### Was der Benutzer erwartet

Er will wissen, welche Parameter es gibt und was die aktuellen Werte sind — ohne die Dokumentation lesen zu muessen.

### Interface

```
$ fotobuch config --help
Show current configuration (YAML + defaults)

Usage: fotobuch config

Options:
  -h, --help  Print help
```

### Verhalten

Gibt die vollstaendig aufgeloeste Konfiguration aus. Felder aus der YAML werden normal angezeigt, fehlende Felder werden mit dem Lib-Default ergaenzt und mit `# default` markiert:

```
$ fotobuch config
config:
  book:
    page_width_mm: 420.0
    page_height_mm: 297.0
    margin_mm: 10.0              # default
    gap_mm: 3.0                  # default
    bleed_mm: 3.0
    bleed_threshold_mm: 5.0      # default
  algorithm:
    timeout_secs: 30             # default
    page_layout:
      population: 200              # default
      generations: 500             # default
      weights:
        size: 1.0                  # default
        coverage: 1.0              # default
        barycenter: 0.5            # default
        order: 0.3                 # default
    book_layout:
      max_images_per_page: 10
      min_images_per_page: 3
      ... # rest aus den MIP-parametern
  preview:
    annotations: true            # default
    page_numbers: true           # default
    max_pixel_per_dimension: 800 # default
```

Die Ausgabe ist gueltige YAML — copy-paste in `fotobuch.yaml` wenn man einen Default ueberschreiben will.

### Zusammenspiel mit CLI-Flags

Aufloesung bei Widerspruch: **CLI-Flag > YAML > Lib-Default**.

```
fotobuch build --population 400   # ueberschreibt YAML fuer diesen Lauf
```

Die Lib exponiert alle Defaults als Konstanten. `cli.rs` verwendet `Option<T>` fuer alle Flags (kein Clap-`default_value`). Die Aufloesung passiert in der Lib:

```rust
fn resolve_ga_config(
    cli: &CliOverrides,
    yaml: &YamlGaConfig,
) -> GaConfig {
    GaConfig {
        population: cli.population
            .or(yaml.population)
            .unwrap_or(defaults::DEFAULT_POPULATION),
        // ...
    }
}
```

### YAML bei `fotobuch new`

`fotobuch new` erzeugt eine vollstaendige YAML mit allen Feldern vorausgefuellt (Pflichtangaben vom CLI, Rest als Defaults). Nichts ist versteckt — der Benutzer sieht sofort was konfigurierbar ist.

---

## 12  Gesamtuebersicht: `fotobuch --help`

```
$ fotobuch --help
A tool for creating photo books from folders of images

Usage: fotobuch <COMMAND>

Commands:
  new      Create a new photobook project
  add      Add photos to the project
  remove   Remove photos or groups from the project
  place    Place unplaced photos into the book
  status   Show project status
  config   Show current configuration (YAML + defaults)
  build    Calculate layout and generate preview PDF (--release for final)
  rebuild  Force re-optimization of pages or page ranges
  history  Show project change history

Options:
  -h, --help     Print help
  -V, --version  Print version
```

---

## 13  Modulstruktur

### Leitprinzip

Die Kommando-Logik lebt in der lib, nicht in der CLI. Grund: Testbarkeit (Integrationstests ohne CLI-Parser) und Wiederverwendbarkeit (TUI/GUI koennte dieselben Funktionen aufrufen).

### Abgrenzung

| Schicht | Verantwortung | Kennt |
|---|---|---|
| `main.rs` | Nur `fn main()`, ruft `cli::run()` | `cli` |
| `cli.rs` | Clap-Definitionen, Argument-Parsing, Ausgabeformatierung | `clap`, `fotobuch::commands` |
| `commands/` | Orchestrierung: ruft `project`, `cache`, `solver`, `output`, `history` in der richtigen Reihenfolge auf | Alle lib-Module |
| Alles andere | Geschaeftslogik, kein CLI-Wissen | Nur eigene Abhaengigkeiten |

`cli.rs` kennt `clap` und `fotobuch::commands`. Die lib kennt kein `clap`.

### Verzeichnisstruktur

```
src/
├── main.rs
├── cli.rs                  # Clap-Parsing, Output-Formatierung
├── lib.rs
├── input.rs
├── commands.rs
├── commands/               # Orchestrierung (ein Modul pro Kommando)
│   ├── new.rs              # Projekt anlegen, git init
│   ├── add.rs              # Fotos scannen, EXIF, Duplikate, YAML update
│   ├── remove.rs           # Fotos/Gruppen entfernen, Slots anpassen
│   ├── status.rs           # Struct-Diff, Aenderungserkennung
│   ├── config.rs           # Aufgeloeste Config ausgeben
│   ├── place.rs            # Unplaced Fotos chronologisch einsortieren
│   ├── build.rs            # Preview-Cache, Solver-Aufrufe, inkrementell
│   ├── rebuild.rs          # Erzwungene Neuberechnung
│   └── history.rs          # git log wrapper
├── models/                 # Datenstrukturen (Photo, Canvas, config ,..) <--- die config sollte hier drin sein, die man im yaml sieht. weiterhin sollte die die einzelnen teile der config auch intern so genutzt werden um den übersetzungsaufwand zu verringern.
│   ├── canvas.rs              
│   ├── ...             
├── solver/
│   ├── page_layout_solver/ # GA-basiert, Slicing-Tree (bestehend)
│   └── book_layout_solver/ # Seitenzuteilung
├── project/                # YAML load/save, Schema-Validierung, Diff-Logik,
├── cache/                  # Preview/Final-Resampling, Wasserzeichen
└── output/                 # Typst-Template-Erzeugung, Kompilierung
```

### Signaturen (Entwurf)

Jedes Command-Modul exponiert eine Funktion die einen Config-Struct nimmt und ein Result zurueckgibt. Kein CLI-Wissen, keine Ausgabeformatierung.

```rust
// commands/new.rs
pub struct NewConfig {
    pub name: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub bleed_mm: f64,
}

pub fn new(parent_dir: &Path, config: &NewConfig) -> Result<PathBuf> { ... }
```

```rust
// commands/add.rs
pub struct AddConfig {
    pub paths: Vec<PathBuf>,
    pub allow_duplicates: bool,
}

pub struct AddResult {
    pub groups_added: Vec<GroupSummary>,
    pub duplicates_skipped: usize,
    pub warnings: Vec<String>,
}

pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> { ... }
```

```rust
// commands/remove.rs
pub struct RemoveConfig {
    pub patterns: Vec<String>,
    pub keep_files: bool,
}

pub struct RemoveResult {
    pub photos_removed: usize,
    pub pages_affected: Vec<usize>,
    pub groups_removed: Vec<String>,
}

pub fn remove(project_root: &Path, config: &RemoveConfig) -> Result<RemoveResult> { ... }
```

```rust
// commands/place.rs
pub struct PlaceConfig {
    pub filter: Option<String>,
    pub into_page: Option<usize>,
}

pub struct PlaceResult {
    pub photos_placed: usize,
    pub pages_affected: Vec<usize>,
}

pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<PlaceResult> { ... }
```

```rust
// commands/build.rs
pub struct BuildConfig {
    pub release: bool,
    pub pages: Option<Vec<usize>>,
}

pub struct BuildResult {
    pub pdf_path: PathBuf,
    pub num_pages: usize,
    pub pages_rebuilt: Vec<usize>,
    pub pages_swapped: Vec<usize>,
    pub dpi_warnings: Vec<DpiWarning>,  // nur bei --release
}

pub fn build(project_root: &Path, config: &BuildConfig) -> Result<BuildResult> { ... }
```

```rust
// commands/rebuild.rs
pub enum RebuildScope {
    All,
    Page(usize),
    Range { start: usize, end: usize, flex: usize },
}

pub fn rebuild(project_root: &Path, scope: RebuildScope) -> Result<BuildResult> { ... }
```

```rust
// commands/config.rs
pub fn config(project_root: &Path) -> Result<ResolvedConfig> { ... }
```

```rust
// commands/status.rs
pub fn status(project_root: &Path, page: Option<usize>) -> Result<StatusReport> { ... }
```

```rust
// commands/history.rs
pub fn history(project_root: &Path) -> Result<Vec<HistoryEntry>> { ... }
```

Die CLI-Schicht (`cli.rs`) wandelt Clap-Args in diese Structs um, ruft die Funktion auf, und formatiert das Result fuer die Konsole:

```rust
// cli.rs (Beispiel)
fn handle_build(args: &BuildArgs) -> Result<()> {
    let config = BuildConfig {
        release: args.release,
        pages: args.pages.clone(),
    };
    let result = fotobuch::commands::build(&project_root, &config)?;

    if result.pages_rebuilt.is_empty() && result.pages_swapped.is_empty() {
        println!("  Nothing to do. Layout is up to date.");
    } else {
        for page in &result.pages_rebuilt {
            println!("  Rebuilt page {}", page);
        }
        println!("  Wrote {} ({} pages)", result.pdf_path.display(), result.num_pages);
    }
    Ok(())
}
```

### Abhaengigkeiten zwischen lib-Modulen

```
commands/ ──→ project/   (YAML load/save, Diff)
         ──→ cache/      (Preview/Final-Resampling)
         ──→ solver/     (Page-Layout, Book-Layout)
         ──→ output/     (Typst-Template, Kompilierung)
         ──→ history/    (Git-Snapshots)

project/ ──→ model/
cache/   ──→ model/
solver/  ──→ model/      (bestehend, keine Abhaengigkeit auf project/cache/output)
output/  ──→ model/
history/ ──→ (nur std::process::Command)
```

`solver/` hat keine Abhaengigkeit auf `project/`, `cache/` oder `output/` — das bleibt wie gehabt. Die `commands/`-Module sind die einzige Schicht, die alle anderen zusammenbringt.

---

## 14  letzte geklärte fragen

- **`fotobuch open`** als Shortcut fuer `typst watch fotobuch_preview.typ` + Editor oeffnen.
