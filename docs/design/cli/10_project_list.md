# `fotobuch project list`

## CLI-Interface

```text
$ fotobuch project list --help
List all photobook projects

Usage: fotobuch project list

Options:
  -h, --help  Print help
```

## Verhalten

Listet alle vorhandenen Fotobuch-Projekte im Repository. Sucht nach `fotobuch/*`-Branches und markiert das aktuelle Projekt.

## Ausgabe

```text
  urlaub        fotobuch/urlaub
* hochzeit      fotobuch/hochzeit   (current)
  geburtstag    fotobuch/geburtstag
```

`*` und `(current)` markieren den aktuell ausgecheckten Branch.

## Fehlerbehandlung

| Situation                   | Verhalten                                  |
| --------------------------- | ------------------------------------------ |
| Kein Git-Repository         | Fehler: `Not a git repository`             |
| Keine `fotobuch/*`-Branches | Leere Liste, CLI zeigt `No projects found` |
| Detached HEAD               | `is_current` ist für alle Projekte `false` |
