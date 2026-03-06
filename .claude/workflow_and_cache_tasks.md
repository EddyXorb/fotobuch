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
├── mein_buch.yaml                # Projektzustand (→ Abschnitt 2)
├── mein_buch_preview.typ         # Typst-Template (Preview)
├── mein_buch_preview.pdf
├── mein_buch_final.typ           # Typst-Template (Final)
├── mein_buch_final.pdf
├── .fotobuch/
│   └── mein_buch/                # Projektname als Unterordner (CLI: --project)
│       ├── cache/
│       │   ├── preview/          # Heruntergerechnete Bilder + Wasserzeichen
│       │   │   └── <rel_path>    # gleicher relativer Pfad wie Original
│       │   └── final/            # 300-DPI-Bilder für Druckexport
│       │       └── <rel_path>    # gleicher relativer Pfad wie Original
│       └── history/              # Git-Repo für YAML-Versionierung (→ Abschnitt 3)
│           ├── .git/
│           └── mein_buch.yaml    # Kopie, wird bei jedem Solver-Lauf committet
├── 2024-01-15_Urlaub/
│   ├── IMG_001.jpg
│   └── IMG_002.jpg
└── 2024-02-20_Geburtstag/
    └── ...
```

**Projektname:** Default `mein_buch`, konfigurierbar via `--project <name>`. Ermöglicht mehrere Bücher aus denselben Quellordnern (z.B. `--project highlights` vs. `--project komplett`).

**Warum Dateien am Root?** Typst kann nur relative Pfade in Unterverzeichnisse auflösen (`#image("...")` ohne `../`). Da die `.typ`-Dateien am Root liegen, können sie sowohl auf `.fotobuch/mein_buch/cache/preview/...` als auch auf die Originalordner `2024-01-15_Urlaub/...` zugreifen. Außerdem sind YAML und PDF für den Benutzer direkt sichtbar — nicht in einem versteckten Ordner versteckt.

Relative Pfade bleiben erhalten:

| Original | Preview | Final |
|---|---|---|
| `2024-01-15_Urlaub/IMG_001.jpg` | `.fotobuch/mein_buch/cache/preview/2024-01-15_Urlaub/IMG_001.jpg` | `.fotobuch/mein_buch/cache/final/2024-01-15_Urlaub/IMG_001.jpg` |

### 1.2  Preview-Cache

**Wann:** Zu Beginn jedes Optimierungslaufs. Nur fehlende/veraltete Bilder werden erzeugt (mtime-Vergleich mit Original).

**Vorgehen:**

1. Original einlesen (read-only).
2. Zielpixelgröße berechnen: z.B. längste Kante = 1200 px (konfigurierbar, sinnvoller Default für schnelle Typst-Kompilierung).
3. Downsampling via `image` crate, `FilterType::Lanczos3` bei Faktor ≤ 2, `FilterType::Triangle` bei Faktor > 2.
4. Wasserzeichen "PREVIEW" auftragen — diagonal über das gesamte Bild, halbtransparent, gut sichtbar. Umsetzung: mit `imageproc` crate Text rendern, oder ein vorgefertigtes PNG-Overlay alpha-blenden.
5. Als JPEG (Qualität ~85) unter dem Spiegelpfad in `.fotobuch/<project>/cache/preview/` abspeichern.

**Zweck des Wasserzeichens:** Verhindert versehentliches Versenden eines Preview-PDFs an Saal Digital. Im finalen PDF sind keine Wasserzeichen.

### 1.3  Final-Cache

**Wann:** Explizit vom Benutzer ausgelöst, nachdem das Layout finalisiert ist (`--export-final` o.ä.).

**Vorgehen:**

1. Für jedes Foto die platzierte Größe in mm aus dem Layout lesen.
2. Ziel-Pixelgröße für 300 DPI berechnen: `px = mm / 25.4 * 300`.
3. Wenn das Original kleiner ist als die Zielgröße → Original verwenden (kein Upsampling), Warnung loggen.
4. Downsampling analog zu Preview (Lanczos/Triangle), aber ohne Wasserzeichen.
5. JPEG-Qualität 95 (oder konfigurierbar).
6. Unter Spiegelpfad in `.fotobuch/<project>/cache/final/` abspeichern.

### 1.4  Typst-Pfadauflösung

Die `.typ`-Dateien liegen am Root, daher sind alle Pfade direkt als Unterverzeichnisse erreichbar:

```typst
// Preview
#image(".fotobuch/mein_buch/cache/preview/2024-01-15_Urlaub/IMG_001.jpg", ...)

// Final
#image(".fotobuch/mein_buch/cache/final/2024-01-15_Urlaub/IMG_001.jpg", ...)
```

Im YAML wird nur der Foto-Relativpfad gespeichert (`2024-01-15_Urlaub/IMG_001.jpg`). Das Typst-Template setzt den Cache-Prefix davor.

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
2. Das Layout manuell anpassen (Fotos verschieben, Seitenzuteilung ändern).
3. Betroffene Seiten erneut optimieren lassen.
4. Schritte 2–3 wiederholen bis zufrieden.

Dafür braucht es eine persistierte Repräsentation des Projekt- und Layout-Zustands, die sowohl vom Solver gelesen/geschrieben als auch vom Benutzer bearbeitet werden kann.

### 2.2  Ansätze im Vergleich

#### Ansatz A: YAML als Source of Truth, Typst wird generiert

```
mein_buch.yaml  ──→  Solver liest/schreibt  ──→  .typ wird generiert  ──→  typst compile  ──→  PDF
       ↑                                             (nur Output,
       │                                              wird überschrieben)
  Benutzer editiert
  YAML direkt
```

**Vorteile:**

- Klare Trennung: YAML = Daten, `.typ` = Darstellung.
- YAML ist trivial zu parsen/serialisieren in Rust (serde_yaml).
- Solver kann YAML direkt lesen, einzelne Seiten neu rechnen, YAML zurückschreiben.
- Keine fragile Typst-Rückwärts-Parsierung nötig.
- Alle Metadaten (Gruppen, area_weight, Originalpfade, GA-Parameter) leben an einem Ort.

**Nachteile:**

- Benutzer muss YAML editieren, nicht Typst. Weniger intuitiv für Position-Tweaks.
- Manuelle mm-Wert-Änderungen im YAML sind blind (kein visuelles Feedback bis zum nächsten `typst compile`).
- Zwei Dateien (YAML + .typ) statt einer, wobei die .typ bei jeder Änderung neu generiert wird.

#### Ansatz B: Typst mit `#yaml()` als Template

```
layout.yaml  ──→  fotobuch.typ liest via #yaml("mein_buch.yaml")  ──→  typst compile  ──→  PDF
       ↑                    (statisches Template,
       │                     wird nie generiert)
  Benutzer editiert
  YAML direkt
```

Die `.typ`-Datei ist ein festes Template, das `#yaml()` aufruft:

```typst
#let data = yaml("mein_buch.yaml")
#let book = data.book
#let cache_prefix = ".fotobuch/mein_buch/cache/preview/"  // bzw. "cache/final/"

#set page(
  width: book.page_width_mm * 1mm,
  height: book.page_height_mm * 1mm,
  margin: 0pt,
)

#for (i, page) in data.pages.enumerate() {
  if i > 0 { pagebreak() }
  for p in page.placements {
    place(top + left,
      dx: p.x_mm * 1mm,
      dy: p.y_mm * 1mm,
      block(
        width: p.width_mm * 1mm,
        height: p.height_mm * 1mm,
        clip: true,
        image(cache_prefix + p.photo, width: p.width_mm * 1mm, height: p.height_mm * 1mm, fit: "cover")
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

#### Ansatz C: Typst-Datei wird generiert, Rück-Parsing bei Re-Optimierung

```
Solver  ──→  .typ generieren  ──→  Benutzer editiert .typ  ──→  Solver parst .typ zurück  ──→  re-optimiert
```

**Vorteile:**

- Nur eine Datei.
- Benutzer editiert direkt das, was er sieht.

**Nachteile:**

- Fragiles Rück-Parsing von Typst-Syntax (Regex/Custom Parser). Jede Benutzer-Änderung am Formatting kann den Parser brechen.
- Metadaten (Gruppen, Gewichte, Originalpfade) müssen als Kommentare in der .typ kodiert werden — hässlich und fehleranfällig.
- Praktisch unwartbar sobald das Template komplexer wird.

### 2.3  Empfehlung

**Ansatz B (YAML + Typst-Template)** ist der sauberste. Das Template wird einmal erstellt und danach nur bei Feature-Erweiterungen angepasst. Der Solver-Code hat keinen Typst-String-Builder mehr, sondern schreibt nur YAML. Das YAML-Schema ist versioniert und validierbar.

Für den Benutzer-Komfort: `typst watch` + YAML-Editor (oder perspektivisch ein kleines TUI/GUI) ergibt einen brauchbaren Feedback-Loop.

Ansatz C ist explizit nicht empfohlen.

### 2.4  YAML-Schema (Entwurf)

```yaml
version: 1
project: "mein_buch"

book:
  page_width_mm: 297.0
  page_height_mm: 210.0
  margin_mm: 10.0
  gap_mm: 3.0
  bleed_mm: 3.0
  bleed_threshold_mm: 5.0

ga:
  population: 200
  generations: 500
  timeout_secs: 30
  weights:
    size: 1.0
    coverage: 1.0
    barycenter: 0.5
    order: 0.3

groups:
  - name: "2024-01-15_Urlaub"
    photos:
      - path: "2024-01-15_Urlaub/IMG_001.jpg"
        width_px: 6000
        height_px: 4000
        area_weight: 1.0
      - path: "2024-01-15_Urlaub/IMG_002.jpg"
        width_px: 4000
        height_px: 6000
        area_weight: 2.0
  - name: "2024-02-20_Geburtstag"
    photos:
      - path: "2024-02-20_Geburtstag/IMG_010.jpg"
        width_px: 5000
        height_px: 3333
        area_weight: 1.0

pages:
  - page: 1
    # Verweis auf welche Fotos (by path) + deren Platzierung
    placements:
      - photo: "2024-01-15_Urlaub/IMG_001.jpg"
        x_mm: -3.0
        y_mm: -3.0
        width_mm: 148.5
        height_mm: 216.0
      - photo: "2024-01-15_Urlaub/IMG_002.jpg"
        x_mm: 151.5
        y_mm: 10.0
        width_mm: 135.5
        height_mm: 190.0
  - page: 2
    placements:
      - photo: "2024-02-20_Geburtstag/IMG_010.jpg"
        x_mm: 10.0
        y_mm: 10.0
        width_mm: 277.0
        height_mm: 190.0
```

**Anmerkungen zum Schema:**

- `photo`-Felder referenzieren via `path` (Unique Key, relativ zum Project Root).
- `groups` enthält die Foto-Metadaten; `pages[].placements` enthält nur Platzierungsdaten + Pfad-Referenz.
- `project`-Feld bestimmt den Cache-Unterordner: `.fotobuch/<project>/cache/...`
- Bei Re-Optimierung einer Seite: Solver liest `pages[i].placements` → extrahiert die `photo`-Pfade → findet die Metadaten in `groups` → lässt GA laufen → schreibt `pages[i].placements` zurück.

### 2.5  Re-Optimierungs-Workflow

```
1. Benutzer editiert mein_buch.yaml:
   - Verschiebt Foto X von page 2 nach page 3
     (entfernt Eintrag aus pages[1].placements, fügt in pages[2].placements ein)
   - Oder: ändert area_weight eines Fotos in groups

2. Benutzer ruft auf:
   fotobuch re-optimize --project mein_buch --pages 2,3

3. Solver:
   a) Liest mein_buch.yaml
   b) Git-Snapshot: "pre: re-optimize pages 2,3" (→ Abschnitt 3)
   c) Für jede angegebene Seite:
      - Sammelt Foto-Metadaten aus groups anhand der placement-Pfade
      - Ruft run_ga(photos_slice, canvas, ga_config) auf
      - Schreibt neue x_mm/y_mm/width_mm/height_mm zurück in pages[i].placements
   d) Schreibt mein_buch.yaml
   e) Git-Snapshot: "post: re-optimize pages 2,3 (cost: 0.0842)"

4. typst compile holt automatisch die neuen Werte via #yaml()
```

**Offene Frage:** Soll `re-optimize` ohne `--pages` alle Seiten neu rechnen? Vermutlich ja, mit der Option `--pages` als Filter. `--project` ist immer Pflicht (oder Default `mein_buch`).

### 2.6  Tasks (Projektzustand)

| # | Task | Abhängigkeit | Aufwand |
|---|---|---|---|
| P1 | YAML-Schema definieren + `serde`-Structs (`ProjectState`, `BookConfig`, `GroupDef`, `PageDef`, `PlacementDef`) | — | M |
| P2 | `project::load(path) → ProjectState` + `project::save(state, path)` mit Schema-Validierung. Dateipfad: `<project>.yaml` am Root. | P1 | S |
| P3 | `project::init(photo_dirs, config) → ProjectState` — Scan + initiale Gruppierung, noch keine Platzierungen | P2 | M |
| P4 | Typst-Template erstellen (`<project>_preview.typ` / `<project>_final.typ`), das `#yaml()` konsumiert; Unterschied nur im Cache-Prefix | P1 | M |
| P5 | `run_ga`-Integration: Ergebnis in `ProjectState.pages[i].placements` schreiben | P1, P2 | S |
| P6 | CLI-Subcommand `init --project <n>` — Projekt anlegen, Previews erzeugen, erste Optimierung | P3, C1, P5 | M |
| P7 | CLI-Subcommand `re-optimize --project <n> [--pages N,M]` — liest YAML, rechnet Seiten neu, schreibt YAML | P5 | M |
| P8 | CLI-Subcommand `export-final --project <n>` — Final-Cache erzeugen, Final-PDF kompilieren | C4, P4 | M |

---

## 3  Git History Tracking

### 3.1  Konzept

Jeder Solver-Aufruf erzeugt automatisch zwei Git-Commits: einen vor und einen nach der Optimierung. Das ermöglicht `git diff` zwischen beliebigen Zuständen, Undo via `git checkout`, und eine vollständige History aller Layout-Änderungen — auch manueller Edits am YAML.

### 3.2  Isoliertes Repo

Das Git-Repo liegt in `.fotobuch/<project>/history/` und trackt ausschließlich eine Kopie der YAML-Datei. Damit ist es vom Rest des Dateisystems entkoppelt — kein Risiko einer Kollision mit bestehenden Repos am Project Root.

```
.fotobuch/mein_buch/history/
├── .git/
└── mein_buch.yaml    # Kopie der Root-YAML
```

### 3.3  Ablauf

```
fotobuch re-optimize --project mein_buch --pages 2,3

  1. cp mein_buch.yaml → .fotobuch/mein_buch/history/mein_buch.yaml
  2. git commit -m "pre: re-optimize pages 2,3"
  3. ... Solver läuft, schreibt mein_buch.yaml ...
  4. cp mein_buch.yaml → .fotobuch/mein_buch/history/mein_buch.yaml
  5. git commit -m "post: re-optimize pages 2,3 (cost: 0.0842)"
```

Bei `init` gibt es nur einen Post-Commit: `"init: 47 photos, 12 pages"`.

Manuelle Edits des Benutzers werden beim nächsten Solver-Aufruf im Pre-Commit erfasst — die Diff zeigt dann genau was der Benutzer verändert hat vs. was der Solver produziert.

### 3.4  Commit-Messages

Strukturierte Messages für maschinelles Parsen:

| Anlass | Format |
|---|---|
| Init | `init: {n} photos, {m} pages` |
| Pre-Optimize | `pre: re-optimize pages {pages}` |
| Post-Optimize | `post: re-optimize pages {pages} (cost: {total_cost})` |
| Pre-Export | `pre: export-final` |

### 3.5  Restore

Zum Zurücksetzen auf einen früheren Zustand:

```bash
cd .fotobuch/mein_buch/history/
git log --oneline                    # Übersicht
git diff HEAD~2 HEAD -- mein_buch.yaml   # Vergleich
git checkout <hash> -- mein_buch.yaml    # Restore in history/
cp mein_buch.yaml ../../../              # Zurückkopieren an Root
```

Ein CLI-Subcommand `fotobuch history --project mein_buch` könnte `git log` wrappen und `fotobuch restore --project mein_buch <hash>` das Zurückkopieren automatisieren. Das ist aber optional und kann später kommen.

### 3.6  Implementierung

Ein dünner Wrapper `history::snapshot(project_root, project_name, message)`:

1. `history_dir` = `.fotobuch/<project>/history/`
2. Falls `.git/` nicht existiert: `git init` + initialer leerer Commit
3. `cp <project>.yaml` → `history_dir/<project>.yaml`
4. `git add <project>.yaml`
5. `git diff --cached --quiet` → falls keine Änderungen, kein Commit (idempotent)
6. `git commit -m "<message>"`

Umsetzung via `std::process::Command` (kein `libgit2` nötig — die Operationen sind trivial, und `git` ist auf jedem Entwicklersystem vorhanden).

### 3.7  Tasks (Git History)

| # | Task | Abhängigkeit | Aufwand |
|---|---|---|---|
| G1 | `history::init(project_root, project_name)` — Repo anlegen falls nötig | — | S |
| G2 | `history::snapshot(project_root, project_name, message)` — Kopie + Commit | G1 | S |
| G3 | Pre/Post-Snapshot-Aufrufe in `init`, `re-optimize`, `export-final` einbauen | G2, P6, P7, P8 | S |

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
  P8 → C4 → C6                     export-final
```

**Testbarkeit:** P1–P3 und C1–C3 sind vollständig unit-testbar ohne GA. G1–G2 sind unit-testbar mit einem Temp-Dir. P5 ist ein Integrationstest mit dem bestehenden `run_ga`. P4 erfordert einen manuellen visuellen Check (oder Snapshot-Test des generierten PDFs).

---

## 5  Offene Punkte

- **Preview-Auflösung:** 1200 px längste Kante als Default — zu viel? Zu wenig? Muss schnell kompilieren, aber Details erkennbar sein.
- **Wasserzeichen-Implementierung:** `imageproc` hat Textrendering, aber Font-Handling ist limitiert. Alternative: festes PNG-Overlay (einfacher, konsistenter). Oder: Wasserzeichen in Typst selbst rendern (als `#place()` mit `#rotate()` + `#text()`), dann braucht das Bild selbst keins.
- **YAML-Migration:** `version`-Feld im Schema ermöglicht spätere Migrationen. Aber: Migrationscode erst schreiben wenn nötig.
- **`typst watch`-Kompatibilität:** Funktioniert `typst watch` korrekt wenn sich das YAML ändert? Typst trackt Dateiabhängigkeiten via `#yaml()` — sollte funktionieren, muss getestet werden.
- **Paralleles Resizing:** Preview-Erzeugung für 500+ Bilder profitiert von `rayon`. Aufwand gering, Effekt groß.
- **`.gitignore`:** `.fotobuch/` sollte ignoriert werden (nur Cache + History). Die YAML-, `.typ`- und PDF-Dateien am Root können versioniert werden falls gewünscht — aber die History in `.fotobuch/` ist der primäre Undo-Mechanismus.
- **`git`-Abhängigkeit:** Der History-Mechanismus setzt `git` im PATH voraus. Falls nicht vorhanden → Warnung loggen, History überspringen. Kein harter Fehler.
