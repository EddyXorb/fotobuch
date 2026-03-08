# Implementation Plan: `fotobuch place`

Stand: 2026-03-08

## Überblick

Fügt unplaced Fotos chronologisch ins bestehende Layout ein. Kein Solver-Aufruf, kein Balancing — nur Zuweisung zu Seiten. Betroffene Seiten brauchen danach `fotobuch build` oder `fotobuch rebuild`.

## Abhängigkeiten

- `dto_models::ProjectState` load/save (vorhanden)
- `git::commit_if_changed` — `git2`-basiert (aus Build-Plan)
- `project::diff::build_photo_index` — Photo-Lookup (aus Build-Plan)
- `regex` — bereits in Cargo.toml

**Keine neuen Crates.**

## Abgrenzung

`place` verändert **nur** `layout[].photos` (die Photo-ID-Listen). Es fasst `layout[].slots` **nicht** an — die Slots werden durch die neuen Fotos ungültig und die Seite braucht einen Rebuild.

`place` ruft **keinen Solver** auf. Es ist eine reine YAML-Manipulation.

---

## Ablauf ohne `--into`

1. **Unplaced Fotos finden** (in `state.photos`, nicht in `state.layout`)
2. **Filter anwenden** falls `--filter` gesetzt (Regex auf `photo.source`)
3. Falls keine unplaced Fotos → `PlaceResult { photos_placed: 0, .. }`, kein Commit
4. Unplaced Fotos nach `timestamp` sortieren
5. Für jedes unplaced Foto: **passende Seite finden** via `find_target_page()`
6. Photo-ID an `layout[page].photos` anhängen
7. **YAML schreiben**, **Git commit**: `place: {n} photos onto pages {affected}`

## Ablauf mit `--into <page>`

1. **Unplaced Fotos finden** + **Filter anwenden**
2. Falls keine → kein Commit
3. Alle matchenden Foto-IDs an `layout[page-1].photos` anhängen
4. **YAML schreiben**, **Git commit**: `place: {n} photos onto page {page}`

---

## Algorithmus: Passende Seite finden

Die chronologische Einsortierung basiert auf dem Timestamp des Fotos relativ zu den Zeiträumen der bestehenden Seiten.

**Definitionen:**
- `page_range(page)` = `(min_timestamp, max_timestamp)` der platzierten Fotos auf der Seite
- `distance(photo_ts, page)` = minimale Zeitdistanz zum nächsten Rand des Seitenbereichs

**Regeln:**
1. Foto-Timestamp liegt **innerhalb** eines Seitenbereichs → diese Seite
2. Foto-Timestamp liegt **zwischen** zwei Seiten → Seite mit kleinstem Abstand
3. Foto-Timestamp liegt **vor** allen Seiten → erste Seite
4. Foto-Timestamp liegt **nach** allen Seiten → letzte Seite
5. Bei Gleichstand → frühere Seite bevorzugen

```rust
/// Findet die Seite deren Zeitraum am besten zum Foto-Timestamp passt.
///
/// Seiten-Zeitbereiche werden vorab berechnet und als sortierte Liste übergeben.
/// Gibt den 0-basierten Index der Zielseite zurück.
fn find_target_page(
    photo_ts: DateTime<Utc>,
    page_ranges: &[(usize, DateTime<Utc>, DateTime<Utc>)], // (page_idx, min_ts, max_ts)
) -> usize {
    // Innerhalb eines Bereichs?
    for &(idx, min_ts, max_ts) in page_ranges {
        if photo_ts >= min_ts && photo_ts <= max_ts {
            return idx;
        }
    }

    // Zwischen Bereichen: nächsten Rand finden
    page_ranges.iter()
        .min_by_key(|&&(_, min_ts, max_ts)| {
            let dist_to_min = (photo_ts - min_ts).num_seconds().unsigned_abs();
            let dist_to_max = (photo_ts - max_ts).num_seconds().unsigned_abs();
            dist_to_min.min(dist_to_max)
        })
        .map(|&(idx, _, _)| idx)
        .unwrap_or(0)
}
```

---

## Signaturen und Strukturen

### `src/commands/place.rs`

```rust
use anyhow::Result;
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

use crate::dto_models::{PhotoFile, ProjectState};
use crate::project::diff::build_photo_index;

/// Konfiguration — unverändert gegenüber bestehendem Stub.
#[derive(Debug, Clone)]
pub struct PlaceConfig {
    /// Regex-Pattern auf photo.source (optional)
    pub filter: Option<String>,
    /// Alle matchenden Fotos auf diese Seite (1-basiert, optional)
    pub into_page: Option<usize>,
}

/// Ergebnis — unverändert gegenüber bestehendem Stub.
#[derive(Debug)]
pub struct PlaceResult {
    pub photos_placed: usize,
    pub pages_affected: Vec<usize>,  // 1-basiert
}

/// Place unplaced photos into the book.
pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<PlaceResult> {
    let mut state = ProjectState::load(&project_root.join("fotobuch.yaml"))?;

    // Validierung
    if state.layout.is_empty() {
        anyhow::bail!("No layout yet. Run `fotobuch build` first.");
    }
    if let Some(page) = config.into_page {
        if page == 0 || page > state.layout.len() {
            anyhow::bail!(
                "Invalid page {} (layout has {} pages)",
                page, state.layout.len()
            );
        }
    }

    // 1. Unplaced Fotos finden
    let unplaced = find_unplaced(&state);
    if unplaced.is_empty() {
        return Ok(PlaceResult { photos_placed: 0, pages_affected: vec![] });
    }

    // 2. Filter anwenden
    let filtered = apply_filter(&unplaced, config.filter.as_deref())?;
    if filtered.is_empty() {
        return Ok(PlaceResult { photos_placed: 0, pages_affected: vec![] });
    }

    // 3. Platzieren
    let pages_affected = if let Some(page) = config.into_page {
        place_into_page(&mut state, &filtered, page - 1)
    } else {
        place_chronologically(&mut state, &filtered)
    };

    let photos_placed = filtered.len();

    // 4. YAML + Git
    state.save(&project_root.join("fotobuch.yaml"))?;
    let pages_str = format_page_list(&pages_affected);
    git::commit_if_changed(project_root, &format!(
        "place: {photos_placed} photos onto {pages_str}"
    ))?;

    Ok(PlaceResult {
        photos_placed,
        pages_affected,
    })
}
```

#### Unplaced Fotos finden

```rust
/// Findet alle Fotos die in state.photos aber nicht in state.layout sind.
/// Gibt (photo_id, source, timestamp) Tupel zurück, sortiert nach timestamp.
fn find_unplaced(state: &ProjectState) -> Vec<UnplacedPhoto> {
    let placed_ids: HashSet<&str> = state.layout.iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();

    let mut unplaced: Vec<UnplacedPhoto> = state.photos.iter()
        .flat_map(|g| g.files.iter().map(|f| UnplacedPhoto {
            id: f.id.clone(),
            source: f.source.clone(),
            timestamp: f.timestamp,
        }))
        .filter(|f| !placed_ids.contains(f.id.as_str()))
        .collect();

    unplaced.sort_by_key(|f| f.timestamp);
    unplaced
}

struct UnplacedPhoto {
    id: String,
    source: String,
    timestamp: DateTime<Utc>,
}
```

#### Filter

```rust
/// Filtert unplaced Fotos via Regex auf photo.source.
fn apply_filter<'a>(
    photos: &'a [UnplacedPhoto],
    pattern: Option<&str>,
) -> Result<Vec<&'a UnplacedPhoto>> {
    match pattern {
        None => Ok(photos.iter().collect()),
        Some(pat) => {
            let re = Regex::new(pat)
                .context(format!("Invalid filter pattern: {pat}"))?;
            Ok(photos.iter().filter(|p| re.is_search(&p.source)).collect())
        }
    }
}
```

#### Chronologische Platzierung

```rust
/// Platziert Fotos chronologisch auf die passenden Seiten.
/// Gibt betroffene Seiten zurück (1-basiert, dedupliziert, sortiert).
fn place_chronologically(
    state: &mut ProjectState,
    photos: &[&UnplacedPhoto],
) -> Vec<usize> {
    // Seiten-Zeitbereiche vorab berechnen
    let photo_index = build_photo_index(state);
    let page_ranges = compute_page_ranges(state, &photo_index);

    let mut affected: HashSet<usize> = HashSet::new();

    for photo in photos {
        let page_idx = find_target_page(photo.timestamp, &page_ranges);
        state.layout[page_idx].photos.push(photo.id.clone());
        affected.insert(page_idx + 1);
    }

    let mut result: Vec<usize> = affected.into_iter().collect();
    result.sort();
    result
}

/// Berechnet (page_idx, min_timestamp, max_timestamp) für jede Seite.
/// Seiten ohne Timestamps werden übersprungen.
fn compute_page_ranges(
    state: &ProjectState,
    photo_index: &HashMap<&str, (&PhotoFile, &str)>,
) -> Vec<(usize, DateTime<Utc>, DateTime<Utc>)> {
    state.layout.iter()
        .enumerate()
        .filter_map(|(idx, page)| {
            let timestamps: Vec<DateTime<Utc>> = page.photos.iter()
                .filter_map(|id| photo_index.get(id.as_str()))
                .map(|(pf, _)| pf.timestamp)
                .collect();
            if timestamps.is_empty() {
                return None;
            }
            let min = *timestamps.iter().min().unwrap();
            let max = *timestamps.iter().max().unwrap();
            Some((idx, min, max))
        })
        .collect()
}

/// Findet die Seite deren Zeitraum am besten zum Foto-Timestamp passt.
fn find_target_page(
    photo_ts: DateTime<Utc>,
    page_ranges: &[(usize, DateTime<Utc>, DateTime<Utc>)],
) -> usize {
    // Innerhalb eines Bereichs?
    for &(idx, min_ts, max_ts) in page_ranges {
        if photo_ts >= min_ts && photo_ts <= max_ts {
            return idx;
        }
    }

    // Nächsten Rand finden, bei Gleichstand frühere Seite
    page_ranges.iter()
        .min_by(|a, b| {
            let dist_a = min_distance(photo_ts, a.1, a.2);
            let dist_b = min_distance(photo_ts, b.1, b.2);
            dist_a.cmp(&dist_b).then(a.0.cmp(&b.0))
        })
        .map(|&(idx, _, _)| idx)
        .unwrap_or(0)
}

fn min_distance(ts: DateTime<Utc>, min: DateTime<Utc>, max: DateTime<Utc>) -> u64 {
    let to_min = (ts - min).num_seconds().unsigned_abs();
    let to_max = (ts - max).num_seconds().unsigned_abs();
    to_min.min(to_max)
}
```

#### Platzierung auf bestimmte Seite

```rust
/// Platziert alle Fotos auf eine bestimmte Seite.
fn place_into_page(
    state: &mut ProjectState,
    photos: &[&UnplacedPhoto],
    page_idx: usize,  // 0-basiert
) -> Vec<usize> {
    for photo in photos {
        state.layout[page_idx].photos.push(photo.id.clone());
    }
    vec![page_idx + 1]
}
```

#### Hilfs-Formatierung

```rust
/// Formatiert Seitenliste für Commit-Message: "page 5" oder "pages 2, 5, 8"
fn format_page_list(pages: &[usize]) -> String {
    if pages.len() == 1 {
        format!("page {}", pages[0])
    } else {
        let list: Vec<String> = pages.iter().map(|p| p.to_string()).collect();
        format!("pages {}", list.join(", "))
    }
}
```

---

## Implementierungsreihenfolge

Setzt voraus, dass Build-Plan Schritt 3 (git2) abgeschlossen ist.

| #   | Schritt | Abhängig von |
| --- | ------- | ------------ |
| 1 | `find_unplaced`, `UnplacedPhoto` struct | — |
| 2 | `apply_filter` (Regex auf source) | 1 |
| 3 | `compute_page_ranges`, `find_target_page`, `min_distance` | 1 |
| 4 | `place_chronologically`, `place_into_page` | 2, 3 |
| 5 | `place()` Hauptfunktion mit Validierung + Git | 4 |

Jeder Schritt = ein Commit. Tests vor jedem Commit.

## Konventionen

- **Conventional Commits**: z.B. `feat: implement unplaced photo detection`, `feat: implement chronological page assignment`
- **Tests**: Unit-Tests + Integrationstests für jeden Schritt
- **`mod solver` unberührt**: `place` ruft keinen Solver auf — nur YAML-Manipulation
- **Dateigröße**: Bei >300 Zeilen `place.rs` in Submodule aufteilen

## Tests

| Test | Prüft |
| ---- | ----- |
| Unplaced Fotos korrekt gefunden | placed_ids HashSet-Logik |
| Chronologisch: Foto 17.01. → Seite mit 15.01. (näher als 20.01.) | find_target_page Distanzberechnung |
| Chronologisch: Foto vor allen Seiten → erste Seite | Edge case |
| Chronologisch: Foto nach allen Seiten → letzte Seite | Edge case |
| Gleichstand → frühere Seite bevorzugt | Tie-breaking |
| `--into 5`: alle unplaced auf Seite 5, nur Seite 5 in affected | place_into_page |
| `--into 0` oder `--into 999` → Fehler | Validierung |
| `--filter "Urlaub"`: nur Fotos mit "Urlaub" im source werden platziert | Regex-Filter |
| `--filter "[invalid"` → sinnvoller Fehler | Regex-Fehlerbehandlung |
| Keine unplaced Fotos → `photos_placed: 0`, kein Git-Commit | Idempotenz |
| Leeres Layout → Fehler mit Hinweis auf `fotobuch build` | Validierung |
| Slots werden NICHT verändert, nur photos-Liste | Keine Seiteneffekte |
