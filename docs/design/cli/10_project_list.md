# Implementation Plan: `fotobuch project list`

Stand: 2026-03-08

## Überblick

Listet alle vorhandenen Fotobuch-Projekte im Repository. Sucht nach `fotobuch/*`-Branches via `git2` und markiert das aktuelle Projekt.

## CLI-Interface

```text
$ fotobuch project list --help
List all photobook projects

Usage: fotobuch project list

Options:
  -h, --help  Print help
```

## Beispielausgabe

```text
  urlaub        fotobuch/urlaub
* hochzeit      fotobuch/hochzeit   (current)
  geburtstag    fotobuch/geburtstag
```

`*` und `(current)` markieren den aktuell ausgecheckten Branch.

## Ablauf

1. **Repository öffnen** via `git2::Repository::open()`
2. **Branches iterieren**: alle lokalen Branches mit Prefix `fotobuch/` filtern
3. **Aktuellen Branch bestimmen**: `repo.head()` → mit Branch-Liste vergleichen
4. **`Vec<ProjectInfo>` zurückgeben** — CLI-Schicht formatiert die Ausgabe

## Signatur

Lebt in `src/commands/project_list.rs` (bereits definiert in [1_new.md](1_new.md)):

```rust
pub struct ProjectInfo {
    pub name: String,
    pub branch: String,       // "fotobuch/<name>"
    pub is_current: bool,
}

pub fn project_list(project_root: &Path) -> Result<Vec<ProjectInfo>>
```

## Fehlerbehandlung

| Situation | Verhalten |
| --- | --- |
| Kein Git-Repository | Fehler: `Not a git repository` |
| Keine `fotobuch/*`-Branches | Leere Liste, CLI zeigt `No projects found` |
| Detached HEAD | `is_current` ist für alle Projekte `false` |

## Tests

| Test | Prüft |
| --- | --- |
| Repository mit 3 Projekten → 3 Einträge | Branch-Erkennung |
| Aktueller Branch → `is_current: true` | Current-Markierung |
| Keine `fotobuch/*`-Branches → leere Liste | Edge case |
| Andere Branches (z.B. `main`) werden ignoriert | Prefix-Filter |
