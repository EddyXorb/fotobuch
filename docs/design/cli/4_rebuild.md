# `fotobuch rebuild`

## CLI-Interface

```text
$ fotobuch rebuild --help
Force re-layout of pages

Usage: fotobuch rebuild [OPTIONS]

Options:
      --page <N>              Rebuild single page
      --pages <START>..<END>  Rebuild page range
      --flex <N>              Allow ±N pages variance for range rebuild [default: 0]
  -h, --help                  Print help
```

Ohne Argumente: kompletter Neustart aller Seiten.

## Abgrenzung zu `build`

| Aspekt             | `build`                     | `rebuild`                       |
| ------------------ | --------------------------- | ------------------------------- |
| Inkrementell       | Ja (Änderungserkennung)     | Nein (immer erzwungen)          |
| Book-Layout-Solver | Nur beim allerersten Aufruf | Bei Range oder All              |
| Page-Layout-Solver | Nur für geänderte Seiten    | Erzwungen für angegebene Seiten |

## Drei Modi

### Einzelseite (`--page N`)

Erzwingt den SinglePage-Solver für die angegebene Seite. Nur `layout[n].slots` wird verändert, die `photos`-Liste bleibt unverändert. Erfordert ein bestehendes Layout.

### Seitenbereich (`--pages START..END [--flex N]`)

Ruft den MultiPage-Solver auf den Fotos des angegebenen Bereichs auf. Mit `--flex N` darf der Solver ±N Seiten mehr oder weniger erzeugen als der Bereich vorgibt. Nach dem Splice werden alle Seiten renummeriert.

Die Fotos werden zurück in ihre ursprünglichen Gruppen sortiert, damit der Solver die Gruppen-Constraints korrekt anwenden kann.

### Kompletter Neustart (ohne Argumente)

Ruft den MultiPage-Solver auf allen Fotos auf, einschließlich bisher unplatzierter. Funktioniert auch ohne bestehendes Layout.

## Commit-Messages

- `rebuild: page N (cost: X.XXXX)`
- `rebuild: pages START-END (cost: X.XXXX)`
- `rebuild: N pages (cost: X.XXXX)`
