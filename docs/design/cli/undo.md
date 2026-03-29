# `fotobuch undo` / `fotobuch redo`

## Interface

```
fotobuch undo [N]    # N Schritte zurück (default: 1)
fotobuch redo [N]    # N Schritte vorwärts (default: 1)
```

## Mechanik

**Redo-Stack**: `.fotobuch/redo-stack` — eine SHA pro Zeile, neueste zuerst. Liegt in `.fotobuch/`, ist bereits gitignored.

### `undo N`

1. Dirty working tree → auto-commit `wip: before undo`
2. Aktuellen HEAD in `.fotobuch/redo-stack` pushen
3. `git reset --hard HEAD~N`

### `redo N`

1. N SHAs aus `.fotobuch/redo-stack` poppen
2. `git reset --hard <sha>`

### Redo-Stack-Invalidierung

Wenn ein normaler Befehl einen Commit erzeugt (add, build, rebuild, …), wird `.fotobuch/redo-stack` geleert. Klassische Undo-Semantik: nach einer neuen Aktion gibt es kein Redo mehr.

## Ausgabe

```
$ fotobuch undo
  Undone: build: 12 pages (cost: 0.0842)
  Now at: add: 47 photos in 3 groups

$ fotobuch undo 3
  Undone 3 steps. Now at: add: 47 photos in 3 groups

$ fotobuch redo
  Redone: build: 12 pages (cost: 0.0842)
```

## Fehlerbehandlung

- `undo` ohne Commits → Fehler: `Nothing to undo.`
- `redo` ohne Stack-Einträge → Fehler: `Nothing to redo.`
- N > verfügbare Commits → Fehler mit Hinweis auf tatsächliche Tiefe

## Abgrenzung zu `git`

`undo`/`redo` sind Convenience-Wrapper für den Workflow — kein Ersatz für `git`. Wer mehr Kontrolle will, nutzt `git reset`, `git checkout`, `git reflog` direkt.
