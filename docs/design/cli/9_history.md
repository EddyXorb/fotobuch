# `fotobuch history`

## CLI-Interface

```text
$ fotobuch history
```

## Verhalten

Zeigt die Projekthistorie ohne Commit-Hash — nur Datum und Commit-Message.

## Ausgabe

```text
2024-03-07 14:22 +0100  release: 12 pages, 85 photos
2024-03-07 14:15 +0100  build: 12 pages (cost: 0.0842)
2024-03-07 12:03 +0100  add: 47 photos in 3 groups
```

## Fehlerbehandlung

- Kein Git-Repo → leere Liste
- Keine Commits → leere Liste
