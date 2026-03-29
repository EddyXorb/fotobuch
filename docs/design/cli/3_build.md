# `fotobuch build`

## CLI-Interface

```text
$ fotobuch build --help
Build the photobook layout and PDF

Usage: fotobuch build [OPTIONS]

Options:
      --release        Build final high-resolution PDF
      --pages <PAGES>  Only rebuild specific pages (e.g. "2,5,8")
  -h, --help           Print help
```

## Verhalten

Inkrementeller Build: Preview-Cache erzeugen, Solver aufrufen (nur wo nötig), Typst kompilieren.

### Erster Build (Layout leer)

Erzeugt Preview-Cache für alle Fotos, ruft den MultiPage-Solver auf, kompiliert das Preview-PDF und committet das Layout.

### Inkrementeller Build

Prüft Preview-Cache auf fehlende/veraltete Einträge (mtime-Vergleich) und erzeugt sie nach. Erkennt geänderte Seiten via StateManager-Diff. Nur geänderte Seiten werden neu gelayoutet (SinglePage-Solver). Falls keine Änderungen vorliegen: `Nothing to do.` ohne Commit und ohne PDF-Neukompilierung.

Bei ausschließlich kompatiblen Swaps (kein Rebuild nötig): PDF neu kompilieren, aber keinen Solver aufrufen.

### `--pages <N,M,...>`

- **Erster Build**: `--pages` wird ignoriert
- **Inkrementeller Build**: Nur die angegebenen Seiten werden auf Änderungen geprüft und ggf. neu gelayoutet
- **Release**: `--pages` ist nicht erlaubt → Fehler

### `--release`

Setzt voraus, dass das Layout clean ist (kein uncommitted diff). Erzeugt den Final-Cache mit 300 DPI aus den Originaldateien. Kein Upsampling: falls das Original kleiner ist als der Slot, wird es kopiert und eine DPI-Warnung ausgegeben. DPI-Warnungen erscheinen vor der PDF-Kompilierung. Kompiliert `final.typ` (generiert aus `{name}.typ` mit `is_final = true`) zu `final.pdf`.

### Cache-Pfade

- Preview: `.fotobuch/cache/{projektname}/preview/{group}/{local_id}.jpg`
- Final: `.fotobuch/cache/{projektname}/final/{group}/{local_id}.jpg`

Preview-Bilder: längste Kante = `config.preview.max_preview_px` (default 800), JPEG 85%.
Final-Bilder: Slot-Größe × 300 DPI, JPEG 95%, Lanczos3 (Faktor ≤ 2) oder Triangle (Faktor > 2).

Wasserzeichen werden nicht ins Bild eingebaut — das Typst-Preview-Template rendert sie als Overlay.

## Commit-Messages

- Preview-Build: `build: N pages (cost: X.XXXX)`
- Release: `release: N pages, M photos`
