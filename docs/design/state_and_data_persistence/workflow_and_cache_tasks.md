# Workflow, Cache & Git History

## Cache-Struktur

```
<project_root>/
├── {name}.yaml                  # Projektzustand
├── {name}.typ                   # Typst-Template (getrackt)
└── .fotobuch/
    └── cache/
        └── {projektname}/
            ├── preview/         # Downgesampelte Bilder + Wasserzeichen
            └── final/           # 300-DPI-Bilder für Druckexport
```

**Warum Dateien am Root:** Typst kann nur relative Pfade in Unterverzeichnisse auflösen. Template und YAML liegen am Root, daher können `.fotobuch/cache/...`-Pfade direkt referenziert werden.

Cache und PDFs werden von Git ignoriert (`.gitignore`).

## Preview-Cache

Zu Beginn jedes Build-Laufs. Fehlende/veraltete Bilder werden erzeugt (mtime-Vergleich mit Original). Downsampling via `image` crate. Wasserzeichen "PREVIEW" wird im Typst-Template gerendert (kein Bild-Preprocessing).

Auflösung: konfigurierbar, Default längste Kante 800 px.

## Final-Cache

Explizit ausgelöst. Ziel-Pixelgröße: `mm / 25.4 * 300`. Wenn Original kleiner als Zielgröße: Original verwenden + Warnung. JPEG-Qualität 95.

## Typst-Template

Das Template ist statisch; nur das YAML ändert sich:

```typst
#let data = yaml("{name}.yaml")
// Cache-Prefix je nach Preview/Final
```

`typst watch` + YAML editieren → Live-Preview ohne Solver.

Bei `--release`: Kopie `final.typ` mit `is_final = true` erzeugen (nicht getrackt).

## Git History Tracking

Jeder Solver-Aufruf erzeugt automatisch einen Git-Commit. Manuelle Edits werden beim nächsten Öffnen des StateManagers auto-committet.

### Commit-Format

| Anlass            | Format                            |
| ----------------- | --------------------------------- |
| Fotos hinzugefügt | `add: {n} photos in {g} groups`   |
| Build             | `build: {p} pages`                |
| Rebuild           | `rebuild: page {n}`               |
| Manuelle Edits    | `chore: manual edits — {summary}` |

### Restore

```bash
git log --oneline
git diff HEAD~2 HEAD -- {name}.yaml
git checkout <hash> -- {name}.yaml
```

Getrackt werden `{name}.yaml` und `{name}.typ`. Cache und PDFs sind ableitbar.
