# Implementation Plan: `fotobuch place`

Stand: 2026-03-08

## Überblick

Fügt unplaced Fotos chronologisch ins bestehende Layout ein. Kein Solver-Aufruf — nur Zuweisung zu Seiten.

## Abhängigkeiten

- `dto_models::ProjectState` load/save (vorhanden)
- `git::commit` (vorhanden)

## Unplaced Fotos finden

```rust
let placed_ids: HashSet<&str> = state.layout.iter()
    .flat_map(|p| p.photos.iter().map(|s| s.as_str()))
    .collect();

let unplaced: Vec<&PhotoFile> = state.photos.iter()
    .flat_map(|g| &g.files)
    .filter(|f| !placed_ids.contains(f.id.as_str()))
    .collect();
```

## Ablauf ohne `--into`

1. Unplaced Fotos nach `timestamp` sortieren
2. Für jedes unplaced Foto: passende Seite finden
   - Zeitstempel des Fotos mit dem Zeitraum der Seite vergleichen
   - "Zeitraum der Seite" = frühester und spätester Timestamp der platzierten Fotos auf der Seite
   - Foto landet auf der Seite deren Zeitraum am nächsten liegt
   - Bei Gleichstand: Seite mit früheren Fotos bevorzugen
3. `layout[i].photos.push(photo_id)` — keine Slots anpassen (Seite braucht rebuild)

## Ablauf mit `--into <page>`

- Alle unplaced Fotos (nach Filter) direkt an `layout[page-1].photos` anhängen
- Seite braucht rebuild

## Filter `--filter <pattern>`

Pattern-Matching auf Photo-ID: `photo.id.contains(pattern)`. Nur matchende Fotos werden platziert.

## Timestamp-Lookup

```rust
fn photo_timestamp(state: &ProjectState, id: &str) -> Option<DateTime<Utc>> {
    state.photos.iter()
        .flat_map(|g| &g.files)
        .find(|f| f.id == id)
        .map(|f| f.timestamp)
}
```

## Seitenzeit-Bestimmung

```rust
fn page_time_range(state: &ProjectState, page: &LayoutPage) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let timestamps: Vec<_> = page.photos.iter()
        .filter_map(|id| photo_timestamp(state, id))
        .collect();
    Some((*timestamps.iter().min()?, *timestamps.iter().max()?))
}
```

## Fehlerbehandlung

- Kein Layout vorhanden → Fehler: "No layout yet. Run `fotobuch build` first."
- `--into <page>` mit ungültiger Seitennummer → Fehler

## YAML + Git

- `state.save("fotobuch.yaml")`
- `git::commit(project_root, "place: {n} photos")`

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit (z.B. `feat: implement unplaced photo detection`, `feat: implement chronological page assignment`, `test: add integration test for place command`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen — kein Commit mit roten Tests.
- **Tests schreiben**: Für jeden neuen Teilschritt sind Unit-Tests (`#[cfg(test)]` inline) und Integrationstests (`tests/`) Pflicht.
- **`mod solver` unberührt**: Keine Änderungen in `src/solver/`. `place` ruft keinen Solver auf — nur YAML-Manipulation.
- **Dateigröße**: Bei >300 Zeilen `place.rs` in `place/` aufteilen.

## Tests

- Unplaced Fotos werden korrekt gefunden
- Chronologische Einsortierung: Foto vom 17.01. landet auf der Seite mit 15.01.-Fotos (näher als 20.01.)
- `--into 5`: alle unplaced auf Seite 5
- `--filter "Urlaub"`: nur Fotos mit "Urlaub" in der ID werden platziert
- Leeres Layout → Fehler mit Hinweis
