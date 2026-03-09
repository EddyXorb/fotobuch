# Implementation Plan: `fotobuch history`

Stand: 2026-03-08

## Überblick

Dünner Wrapper um `git log`. Zeigt Projekthistorie ohne Hash, nur Datum + Message.

## Abhängigkeiten

- `std::process::Command`
- Keine neuen Crates

## Implementierung

```rust
pub fn history(project_root: &Path) -> Result<Vec<HistoryEntry>> {
    let output = Command::new("git")
        .args(["log", "--format=%ai\t%s"])
        .current_dir(project_root)
        .output()?;

    if !output.status.success() {
        return Ok(Vec::new()); // Kein Git oder keine Commits
    }

    let entries = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let (ts, msg) = line.split_once('\t')?;
            Some(HistoryEntry {
                timestamp: ts.trim().to_string(),
                message: msg.to_string(),
            })
        })
        .collect();

    Ok(entries)
}
```

`%ai` liefert ISO 8601 mit Zeitzone, `%s` die Subject-Zeile. Tab als Trenner verhindert Konflikte mit Leerzeichen in Nachrichten.

## CLI-Ausgabe

```
2024-03-07 14:22 +0100  release: 12 pages, 85 photos
2024-03-07 14:15 +0100  post-build: 12 pages (cost: 0.0842)
...
```

Die CLI-Schicht formatiert den Timestamp auf `YYYY-MM-DD HH:MM` (Timezone optional).

## Fehlerbehandlung

- Kein Git-Repo → leere Liste
- Keine Commits → leere Liste

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement history command via git log`, `test: add unit test for history parsing`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. Alle Implementierungen spielen sich ausschließlich in `src/commands/history.rs` ab.
- **Dateigröße**: `history.rs` wird voraussichtlich klein bleiben; bei >300 Zeilen in `history/` aufteilen.

## Tests

- In tempdir mit initialisiertem Repo: Commits prüfen ob sie erscheinen
- Ohne Repo: leere Liste ohne Fehler
