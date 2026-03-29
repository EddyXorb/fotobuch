# `fotobuch remove`

## CLI-Interface

```text
$ fotobuch remove --help
Remove photos from the project

Usage: fotobuch remove [OPTIONS] <PATTERNS>...

Arguments:
  <PATTERNS>...  Group names or regex patterns on photo source path

Options:
      --keep-files  Remove from layout only (photos stay as unplaced)
  -h, --help        Print help
```

## Verhalten

Entfernt Fotos oder ganze Gruppen aus dem Projekt. Pflegt `photos` und `layout` konsistent. Leere Seiten werden automatisch entfernt und das Layout neu nummeriert.

### Symmetrie

```text
add      <->  remove              (Projekt-Ebene: photos + layout)
place    <->  remove --keep-files (Layout-Ebene: nur layout[].photos)
```

### Pattern-Matching

Jedes Pattern ist entweder ein **exakter Gruppenname** oder eine **Regex auf `photo.source`** (absoluter Pfad). Mehrere Patterns werden mit OR verknüpft.

```text
fotobuch remove "2024-01-15_Urlaub"         # Gruppenname-Match
fotobuch remove "IMG_001\.jpg$"             # Regex auf source
fotobuch remove "Urlaub/IMG_00[1-3]"        # Regex mit Character-Class
fotobuch remove "Urlaub" "Geburtstag"       # Mehrere Patterns (OR)
fotobuch remove --keep-files "IMG_005\.jpg" # Nur aus Layout entfernen
```

### Standard (ohne `--keep-files`)

Photos und korrespondierende Slots werden aus dem Layout entfernt. Leere Seiten werden gelöscht. Fotos werden aus `photos` entfernt, leere Gruppen ebenfalls.

### Mit `--keep-files`

Nur das Layout wird bereinigt (Photos + Slots entfernt). Die Fotos bleiben als "unplaced" in `photos` und können erneut mit `place` platziert werden.

### Slots nach Remove

Verbleibende Slots auf einer betroffenen Seite sind geometrisch veraltet (das Layout war für eine andere Fotozahl optimiert). Die Slots werden beibehalten, damit das Preview-PDF halbwegs brauchbar bleibt. `status` markiert die Seite als "needs rebuild".

### Nichts gematcht

Kein Commit.

## Commit-Messages

- `remove: N photos`
- `remove: N placements from layout (photos kept)`
