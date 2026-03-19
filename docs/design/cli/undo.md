# `fotobuch undo` / `fotobuch redo`

Stand: 2026-03-19

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

### Redo-Stack invalidieren

Wenn ein normaler Befehl einen Commit erzeugt (add, build, rebuild, …), wird `.fotobuch/redo-stack` geleert. Klassische Undo-Semantik: nach einer neuen Aktion gibt es kein Redo mehr.

Implementierung: `StateManager::commit()` oder eine zentrale `git_commit()`-Hilfsfunktion leert den Stack vor dem Commit.

## Fehlerbehandlung

- `undo` ohne Commits → Fehler: `Nothing to undo.`
- `redo` ohne Stack-Einträge → Fehler: `Nothing to redo.`
- N > verfügbare Commits → Fehler mit Hinweis auf tatsächliche Tiefe

## Ausgabe

```
$ fotobuch undo
  Undone: post-build: 12 pages (cost: 0.0842)
  Now at: pre-build: pages 5, 8 modified

$ fotobuch undo 3
  Undone 3 steps. Now at: add: 47 photos in 3 groups

$ fotobuch redo
  Redone: post-build: 12 pages (cost: 0.0842)
```

## Implementierung

```rust
// commands/undo.rs
pub fn undo(project_root: &Path, steps: usize) -> Result<UndoResult>
pub fn redo(project_root: &Path, steps: usize) -> Result<UndoResult>

pub struct UndoResult {
    pub undone_message: String,   // Commit-Message des rückgängig gemachten Commits
    pub current_message: String,  // Commit-Message des neuen HEAD
}
```

Redo-Stack-Verwaltung in einem eigenen Submodul `commands/undo/stack.rs`:

```rust
pub fn push(project_root: &Path, sha: &str) -> Result<()>
pub fn pop_n(project_root: &Path, n: usize) -> Result<Vec<String>>  // gibt gepoopte SHAs zurück
pub fn clear(project_root: &Path) -> Result<()>
```

## Abgrenzung zu `git`

`undo`/`redo` sind Convenience-Wrapper für den Workflow — kein Ersatz für `git`. Wer mehr Kontrolle will, nutzt `git reset`, `git checkout`, `git reflog` direkt.
