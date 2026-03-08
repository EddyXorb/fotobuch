# Implementation Plan: `fotobuch project switch`

Stand: 2026-03-08

## Überblick

Wechselt zum Branch eines anderen Fotobuch-Projekts. Führt `git checkout fotobuch/<name>` via `git2` aus — der Working Tree zeigt danach `<name>.yaml` und `<name>.typ`.

## CLI-Interface

```text
$ fotobuch project switch --help
Switch to another photobook project

Usage: fotobuch project switch <NAME>

Arguments:
  <NAME>  Project name to switch to

Options:
  -h, --help  Print help
```

## Ablauf

1. **Projektname validieren** via `validate_project_name()`
2. **Repository öffnen** via `git2::Repository::open()`
3. **Branch-Existenz prüfen**: `fotobuch/<name>` muss existieren
4. **Uncommitted Changes prüfen**: bei unsaved Changes → Fehler mit Hinweis
5. **Branch wechseln**: `git checkout fotobuch/<name>` via `git2`

## Signatur

Lebt in `src/commands/project/switch.rs`:

```rust
pub fn project_switch(project_root: &Path, name: &str) -> Result<()>
```

## Fehlerbehandlung

| Situation | Verhalten |
| --- | --- |
| Branch `fotobuch/<name>` existiert nicht | Fehler: `Project '<name>' not found. Use 'fotobuch project list' to see available projects.` |
| Uncommitted Changes im Working Tree | Fehler: `Working tree has uncommitted changes. Commit or stash before switching.` |
| Bereits auf dem Ziel-Branch | Hinweis: `Already on project '<name>'` (kein Fehler) |
| Ungültiger Projektname | Fehler aus `validate_project_name()` |

## Tests

| Test | Prüft |
| --- | --- |
| Switch zu existierendem Projekt → HEAD zeigt auf `fotobuch/<name>` | Branch-Wechsel |
| Switch zu nicht-existierendem Projekt → Fehler | Existenz-Check |
| Switch mit uncommitted Changes → Fehler | Dirty-Check |
| Switch auf aktuellen Branch → Hinweis, kein Fehler | Idempotenz |
