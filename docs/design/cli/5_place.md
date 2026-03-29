# `fotobuch place`

## CLI-Interface

```text
$ fotobuch place --help
Assign unplaced photos to pages

Usage: fotobuch place [OPTIONS]

Options:
      --filter <PATTERN>  Regex filter on photo source path
      --into <PAGE>       Place all matching photos onto this page (1-based)
  -h, --help              Print help
```

## Verhalten

Fügt unplaced Fotos (in `photos`, aber nicht in `layout`) ins bestehende Layout ein. Kein Solver-Aufruf — nur YAML-Manipulation. `place` verändert ausschließlich `layout[].photos`, nie `layout[].slots`. Betroffene Seiten sind danach outdated und brauchen `fotobuch build` oder `fotobuch rebuild`.

Erfordert ein bestehendes Layout. Sind keine unplatzierten Fotos vorhanden (nach optionalem Filter), passiert nichts und es wird kein Commit erstellt.

### Chronologische Zuweisung (ohne `--into`)

Fotos werden nach Timestamp sortiert. Für jedes Foto wird die Seite bestimmt, deren bereits platzierte Fotos zeitlich am nächsten liegen:

1. Foto-Timestamp liegt **innerhalb** eines Seitenbereichs → diese Seite
2. Foto-Timestamp liegt **zwischen** zwei Seiten → Seite mit kleinstem Abstand zum Rand
3. Foto-Timestamp liegt **vor** allen Seiten → erste Seite
4. Foto-Timestamp liegt **nach** allen Seiten → letzte Seite
5. Bei Gleichstand → frühere Seite bevorzugen

### Zuweisung auf bestimmte Seite (`--into PAGE`)

Alle matchenden unplatzierten Fotos werden der angegebenen Seite zugewiesen.

### Filter (`--filter PATTERN`)

Regex auf `photo.source` (absoluter Pfad). Nur Fotos mit passendem Pfad werden berücksichtigt.

## Commit-Message

`place: N photos onto page M` oder `place: N photos onto pages M, P, Q`
