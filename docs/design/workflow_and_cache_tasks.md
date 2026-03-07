# Workflow & Cache Pipeline — Task-Dokument

Stand: 2026-03-06

## Kontext

Der Seitenzuteilungs- und Layout-Solver (GA-basiert, Slicing-Tree) ist funktionsfähig.
Dieses Dokument beschreibt die nächsten Schritte, um das Programm als End-to-End-Workflow benutzbar zu machen.

Drei Themenblöcke:

1. **Image Cache Pipeline** — Preview/Final-Bilder, Wasserzeichen, 300-DPI-Export
2. **Projektzustand & Re-Optimierung** — Zustand persistieren, manuelle Anpassungen ermöglichen, betroffene Seiten neu rechnen
3. **Git History Tracking** — Automatische Versionierung des YAML-Zustands vor/nach jedem Solver-Lauf

---

## 1  Image Cache Pipeline

### 1.1  Verzeichnisstruktur

```
<project_root>/
├── .git/                         # Von `fotobuch new` erstellt (→ Abschnitt 3)
├── .gitignore
├── fotobuch.yaml                 # Projektzustand (→ Abschnitt 2)
├── fotobuch_preview.typ          # Typst-Template (Preview)
├── fotobuch_preview.pdf
├── fotobuch_final.typ            # Typst-Template (Final)
├── fotobuch_final.pdf
├── .fotobuch/
│   └── cache/
│       ├── preview/              # Heruntergerechnete Bilder + Wasserzeichen
│       │   └── <rel_path>        # gleicher relativer Pfad wie Original
│       └── final/                # 300-DPI-Bilder für Druckexport
│           └── <rel_path>        # gleicher relativer Pfad wie Original
└── (Fotos liegen extern, werden via absolutem Pfad referenziert)
```

**Ein Projekt pro Verzeichnis.** `fotobuch new mein-buch` erstellt das Verzeichnis und das Projekt darin. Mehrere Buecher aus denselben Fotos = mehrere Verzeichnisse.

**Warum Dateien am Root?** Typst kann nur relative Pfade in Unterverzeichnisse aufloesen (`#image("...")` ohne `../`). Da die `.typ`-Dateien am Root liegen, koennen sie auf `.fotobuch/cache/preview/...` zugreifen. YAML und PDF sind fuer den Benutzer direkt sichtbar — nicht in einem versteckten Ordner versteckt.

Relative Pfade bleiben erhalten:

| Original | Preview | Final |
|---|---|---|
| `2024-01-15_Urlaub/IMG_001.jpg` | `.fotobuch/cache/preview/2024-01-15_Urlaub/IMG_001.jpg` | `.fotobuch/cache/final/2024-01-15_Urlaub/IMG_001.jpg` |

### 1.2  Preview-Cache

**Wann:** Zu Beginn jedes Optimierungslaufs. Nur fehlende/veraltete Bilder werden erzeugt (mtime-Vergleich mit Original).

**Vorgehen:**

1. Original einlesen (read-only).
2. Zielpixelgröße berechnen: z.B. längste Kante = 800 px (konfigurierbar, sinnvoller Default für schnelle Typst-Kompilierung).
3. Downsampling via `image` crate, `FilterType::Lanczos3` bei Faktor ≤ 2, `FilterType::Triangle` bei Faktor > 2.
4. Wasserzeichen "PREVIEW" auftragen — diagonal über das gesamte Bild, halbtransparent, gut sichtbar. Umsetzung: mit `imageproc` crate Text rendern, oder ein vorgefertigtes PNG-Overlay alpha-blenden.
5. Als JPEG (Qualität ~85) unter dem Spiegelpfad in `.fotobuch/cache/preview/` abspeichern.

**Zweck des Wasserzeichens:** Verhindert versehentliches Versenden eines Preview-PDFs an Saal Digital. Im finalen PDF sind keine Wasserzeichen.

### 1.3  Final-Cache

**Wann:** Explizit vom Benutzer ausgelöst, nachdem das Layout finalisiert ist (mit "fotobuch export", siehe cli.design.md).

**Vorgehen:**

1. Für jedes Foto die platzierte Größe in mm aus dem Layout lesen.
2. Ziel-Pixelgröße für 300 DPI berechnen: `px = mm / 25.4 * 300`.
3. Wenn das Original kleiner ist als die Zielgröße → Original verwenden (kein Upsampling), Warnung loggen.
4. Downsampling analog zu Preview (Lanczos/Triangle), aber ohne Wasserzeichen.
5. JPEG-Qualität 95 (oder konfigurierbar).
6. Unter Spiegelpfad in `.fotobuch/cache/final/` abspeichern.

### 1.4  Typst-Pfadauflösung

Die `.typ`-Dateien liegen am Root, daher sind alle Pfade direkt als Unterverzeichnisse erreichbar:

```typst
// Preview
#image(".fotobuch/cache/preview/2024-01-15_Urlaub/IMG_001.jpg", ...)

// Final
#image(".fotobuch/cache/final/2024-01-15_Urlaub/IMG_001.jpg", ...)
```

Im YAML wird nur der Foto-Relativpfad gespeichert (`2024-01-15_Urlaub/IMG_001.jpg`). Das Typst-Template setzt den Cache-Prefix davor.

Wenn im add Gruppen gleichen namens hinzugefügt werden, sollen die Fotos in einer gemeinsamen Gruppe landen.

### 1.5  Tasks (Image Cache)

| # | Task | Abhängigkeit | Aufwand |
|---|---|---|---|
| C1 | `cache::ensure_preview(photos, config) → HashMap<RelPath, CachePath>` — erzeugt fehlende Previews, gibt Mapping zurück | — | M |
| C2 | `cache::watermark(image) → image` — "PREVIEW"-Stempel diagonal über Bild | C1 | S |
| C3 | `cache::compute_final_size(placement, page_size, dpi) → (u32, u32)` — Ziel-Pixel aus mm + DPI | — | S |
| C4 | `cache::export_final(layout, photos, config) → HashMap<RelPath, CachePath>` — erzeugt Final-Bilder basierend auf fertigem Layout | C3 | M |
| C5 | `typst_export` anpassen: Pfade auf Cache verweisen statt auf Originale | C1, C4 | S |
| C6 | CLI-Subcommand `export-final` | C4, C5 | S |

---

## 2  Projektzustand & Re-Optimierung

### 2.1  Problemstellung

Der Benutzer soll:

1. Einen Optimierungslauf starten → PDF/Typst-Output erhalten.
2. Das Layout manuell anpassen bzw. mit Hilfe von cli-commands ("place") Fotos verschieben, Seitenzuteilung ändern.
3. Betroffene Seiten erneut optimieren lassen.
4. Schritte 2–3 wiederholen bis zufrieden.

Dafür braucht es eine persistierte Repräsentation des Projekt- und Layout-Zustands, die sowohl vom Solver gelesen/geschrieben als auch vom Benutzer bearbeitet werden kann.

### 2.1  Gewählter Ansatz

#### Typst mit `#yaml()` als Template

```
layout.yaml  ──→  fotobuch.typ liest via #yaml("fotobuch.yaml")  ──→  typst compile  ──→  PDF
       ↑                    (statisches Template,
       │                     wird nie generiert)
  Benutzer editiert
  YAML direkt
```

Die `.typ`-Datei ist ein festes Template, das `#yaml()` aufruft:

```typst
#let data = yaml("fotobuch.yaml")
#let book = data.book
#let cache_prefix = ".fotobuch/cache/preview/"  // bzw. "cache/final/"

#set page(
  width: book.page_width_mm * 1mm,
  height: book.page_height_mm * 1mm,
  margin: 0pt,
)

#for (pi, page) in data.pages.enumerate() {
  if pi > 0 { pagebreak() }
  for (i, photo) in page.photos.enumerate() {
    let slot = page.layout.at(i)
    place(top + left,
      dx: slot.x_mm * 1mm,
      dy: slot.y_mm * 1mm,
      block(
        width: slot.width_mm * 1mm,
        height: slot.height_mm * 1mm,
        clip: true,
        image(cache_prefix + photo, width: slot.width_mm * 1mm, height: slot.height_mm * 1mm, fit: "cover")
      )
    )
  }
}
```

**Vorteile:**

- Alle Vorteile von Ansatz A.
- Zusätzlich: `.typ`-Datei muss nie neu generiert werden. Nur YAML ändert sich.
- Benutzer kann `typst watch` laufen lassen, YAML editieren → Live-Preview.
- Das Template kann in Typst-Syntax beliebig erweitert werden (Seitennummern, Titel, Hintergrundfarben) ohne den Solver zu berühren.
- Saubere Architektur: Solver kennt nur YAML, Typst kennt nur YAML + Template.

**Nachteile:**

- Typst-Template muss korrekt mit der YAML-Struktur umgehen. Schema-Änderungen erfordern Template-Update.
- Debugging: Fehler im Template vs. Fehler in den Daten kann schwerer unterscheidbar sein.
- Typsts `#yaml()` parsed das File bei jedem Compile. Bei großen Projekten (1000+ Fotos) eventuell relevant, aber vermutlich vernachlässigbar.
- Benutzer, der das Template selbst anpassen will (z.B. Seitenränder, Fonts), muss Typst-Syntax können.

### 2.2  YAML-Schema

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
      - id: "2024-01-15_Urlaub/IMG_002.jpg"
        file: "IMG_002.jpg"
        width_px: 4000
        height_px: 6000
        area_weight: 2.0
  - group: "2024-02-20_Geburtstag"
    sort_key: "2024-02-20T14:00:00"
    files:
      - id: "2024-02-20_Geburtstag/IMG_010.jpg"
        ssource: /home/user/Fotos/2024-02-20_Geburtstag/IMG_010.jpg"
        width_px: 5000
        height_px: 3333
        area_weight: 1.0

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
- `source`: Absoluter Pfad zum Original. Wird fuer Preview-/Final-Resize und Cache-Rebuild benoetigt. Das Original wird nie veraendert.
- `sort_key`: Zeitstempel pro Gruppe fuer die chronologische Reihenfolge (von `add` per Heuristik (frühester timestamp aller hinzugefügten fotos nach erstem add) ermittelt, manuell aenderbar).
- `layout`: Das Buch-Layout. Pro Seite `photos` (Benutzer-Input) und `slots` (Solver-Output).
- `layout[].photos`: Welche Fotos auf der Seite, sortiert nach Ratio aufsteigend. Der `# 0.67`-Kommentar wird beim Schreiben per Post-Processing berechnet (`serde_yaml` unterstuetzt keine Kommentare).
- `layout[].slots`: Berechnete Platzierungen, Index-gekoppelt an `photos`. `slots[i]` ist der Slot fuer `photos[i]`.
- **Tausch innerhalb einer Seite:** Zwei Zeilen in `photos` tauschen, `slots` nicht anfassen. Fotos mit aehnlichem Ratio (Kommentar pruefen) sind problemlos tauschbar.
- **Tausch ueber Seitengrenzen:** Zeile aus `photos` der einen Seite in die andere verschieben (gleicher Index im Ziel). Aehnliches Ratio = kein Rebuild noetig.
- Bei Re-Optimierung einer Seite: Solver liest `layout[i].photos` -> findet Metadaten in `photos` (Top-Level) -> laesst GA laufen -> schreibt `layout[i].slots` neu. Die `photos`-Liste bleibt unangetastet.
- `layout[].page`: ein counter, der nur für den benutzer da ist und dem index +1 in der liste entsprechend sollte; wird nicht fürs rendering verwendet und dient nur als info; d.h. änderungen daran durch benutzer ändern das fotobuch nicht. Wird nach jedem build/rebuild neu gesetzt

### 2.5  Build- und Rebuild-Workflow

**Inkrementeller Build** (Normalfall nach manuellen Edits):

```
1. Benutzer editiert fotobuch.yaml:
   - Tauscht Fotos mit gleichem Ratio zwischen Seiten
     (Zeilen in layout[].photos verschieben)
   - Oder: aendert area_weight eines Fotos in `photos`

2. Benutzer ruft auf:
   fotobuch build

3. Solver:
   a) Liest fotobuch.yaml
   b) Vergleicht mit letztem Commit (Struct-Diff)
   c) Identifiziert Seiten in `layout` die Rebuild brauchen:
      - Ratio-kompatibler Swap -> kein Rebuild, nur PDF neu kompilieren
      - Ratio-Mismatch oder Foto hinzugefuegt/entfernt -> Page-Layout-Solver
   d) Fuer jede betroffene Seite: run_ga() -> schreibt layout[i].slots
   e) YAML schreiben, Git-Commit
   f) typst compile -> Preview-PDF

4. typst watch erkennt Aenderung automatisch via #yaml()
```

**Expliziter Rebuild** (Layout erzwingen oder Bereich neu verteilen):
Siehe cli-design.md für die maßgebliche definition.

```
fotobuch rebuild 5       -> Page-Layout-Solver auf Seite 5 (erzwungen)
fotobuch rebuild 3-7     -> Book-Layout-Solver auf Bereich, dann Page-Layout fuer 3-7
fotobuch rebuild         -> Alles von vorn (wie erster build)
```

Siehe cli_design.md Abschnitte 5-6 fuer Details.

### 2.6  Tasks (Projektzustand)

| # | Task | Abhängigkeit | Aufwand |
|---|---|---|---|
| P1 | YAML-Schema definieren + `serde`-Structs (`ProjectState`, `Config`, `PhotoGroup`, `LayoutPage`, `Slot`) | — | M |
| P2 | `project::load(path) → ProjectState` + `project::save(state, path)` mit Schema-Validierung. Dateipfad: `<project>.yaml` am Root. | P1 | S |
| P3 | `project::init(photo_dirs, config) → ProjectState` — Scan + initiale Gruppierung, noch keine Platzierungen | P2 | M |
| P4 | Typst-Template erstellen (`<project>_preview.typ` / `<project>_final.typ`), das `#yaml()` konsumiert; Unterschied nur im Cache-Prefix | P1 | M |
| P5 | `run_ga`-Integration: Ergebnis in `ProjectState.layout[i].slots` schreiben | P1, P2 | S |
| P6 | CLI-Subcommand `new` — Projekt anlegen, Previews erzeugen, erste Optimierung | P3, C1, P5 | M |
| P7 | CLI-Subcommands `build` (inkrementell) + `rebuild [PAGE|RANGE]` (erzwungen) — siehe cli_design.md | P5 | M |
| P8 | `build --release` — Final-Cache (immer voll aus Originalen), Final-PDF kompilieren | C4, P4 | M |

---

## 3  Git History Tracking

### 3.1  Konzept

Jeder Solver-Aufruf erzeugt automatisch zwei Git-Commits: einen vor und einen nach der Optimierung. Das ermöglicht `git diff` zwischen beliebigen Zuständen, Undo via `git checkout`, und eine vollständige History aller Layout-Änderungen — auch manueller Edits am YAML.

### 3.2  Git am Project Root

Da `fotobuch new` das Projektverzeichnis selbst erstellt, gibt es keine Kollision mit bestehenden Repos. Das Git-Repo liegt direkt im Projekt-Root — kein isoliertes Sub-Repo, kein Kopieren von Dateien.

`.gitignore` (von `fotobuch new` erzeugt):

```gitignore
.fotobuch/
*.pdf
```

Getrackt werden `fotobuch.yaml` und die `.typ`-Templates. Cache und PDFs sind ableitbar.

### 3.3  Ablauf

```
fotobuch rebuild --pages 2,3

  1. git add fotobuch.yaml && git commit -m "pre-rebuild: pages 2,3"
  2. ... Solver läuft, schreibt fotobuch.yaml ...
  3. git add fotobuch.yaml && git commit -m "post-rebuild: pages 2,3 (cost: 0.0842)"
```

Bei `build` gibt es nur einen Post-Commit: `"post-build: 12 pages (cost: 0.0842)"`.

Manuelle Edits des Benutzers werden beim nächsten Solver-Aufruf im Pre-Commit erfasst — die Diff zeigt dann genau was der Benutzer verändert hat vs. was der Solver produziert.

### 3.4  Commit-Messages

Strukturierte Messages für maschinelles Parsen:

| Anlass | Format |
|---|---|
| Projekt erstellt | `new: {W}x{H}mm, {B}mm bleed` |
| Fotos hinzugefuegt | `add: {n} photos in {g} groups` |
| Pre-Build | `pre-build: {n} groups, {m} photos` |
| Post-Build | `post-build: {p} pages (cost: {total_cost})` |
| Pre-Rebuild | `pre-rebuild: pages {pages}` |
| Post-Rebuild | `post-rebuild: pages {pages} (cost: {total_cost})` |
| Release | `release: {p} pages, {n} photos` |

### 3.5  Restore

Zum Zurücksetzen auf einen früheren Zustand:

```bash
git log --oneline                         # Übersicht
git diff HEAD~2 HEAD -- fotobuch.yaml     # Vergleich
git checkout <hash> -- fotobuch.yaml      # Restore
```

Standard-Git-Workflows funktionieren direkt. Ein CLI-Subcommand `fotobuch history` könnte `git log` wrappen, ist aber optional — der Benutzer kennt git.

### 3.6  Implementierung

Ein dünner Wrapper `history::snapshot(project_root, message)`:

1. `git add fotobuch.yaml`
2. `git diff --cached --quiet` → falls keine Änderungen, kein Commit (idempotent)
3. `git commit -m "<message>"`

Umsetzung via  `libgit2`, damit auch auf system lauffähig die kein git installiert haben. Sicherheit hat oberste priorität, dass kein fotobuch verloren geht aus unachtsamkeit, daher muss git erzwungen werden.

### 3.7  Tasks (Git History)

| # | Task | Abhängigkeit | Aufwand |
|---|---|---|---|
| G1 | `history::init(project_root)` — `git init --initial-branch=fotobuch` + `.gitignore` + initialer Commit bei `new` | — | S |
| G2 | `history::snapshot(project_root, message)` — `git add` + Commit | G1 | S |
| G3 | Pre/Post-Snapshot-Aufrufe in `build`, `rebuild`, `build --release` einbauen | G2, P6, P7, P8 | S |

---

## 4  Umsetzungsreihenfolge

```
Phase 1: Grundgerüst
  P1 → P2 → P3                     YAML-Schema + Load/Save + Init
  G1 → G2                          Git History Init + Snapshot

Phase 2: Cache
  C1 → C2                          Preview-Cache + Wasserzeichen
  C3                                DPI-Berechnung

Phase 3: Integration
  P4                                Typst-Template
  P5                                run_ga → YAML-Writeback
  C5                                Typst-Pfade auf Cache

Phase 4: CLI
  P6 → G3                          init (Scan → Preview → Optimize → YAML + PDF + Snapshot)
  P7 → G3                          re-optimize (Pre/Post-Snapshot)
  P8 → C4 → C6                     build --release
```

**Testbarkeit:** P1–P3 und C1–C3 sind vollständig unit-testbar ohne GA. G1–G2 sind unit-testbar mit einem Temp-Dir. P5 ist ein Integrationstest mit dem bestehenden `run_ga`. P4 erfordert einen manuellen visuellen Check (oder Snapshot-Test des generierten PDFs).

---

## 5  Letzte Punkte klären

- **Preview-Auflösung:** 800 px längste Kante als Default ist ok für den anfang. Sollte editierbar sein in der config.
- **Wasserzeichen-Implementierung:** Wasserzeichen in Typst selbst rendern (als `#place()` mit `#rotate()` + `#text()`), dann braucht das Bild selbst keins.
- **Paralleles Resizing:** Preview-Erzeugung für 500+ Bilder profitiert von `rayon`. Aufwand gering, Effekt groß. Also nutzen.
