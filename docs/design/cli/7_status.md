# Implementation Plan: `fotobuch status`

Stand: 2026-03-08

## Überblick

Zeigt Projektstatus: Fotos, Gruppen, Layout-Zusammenfassung, geänderte Seiten seit letztem Build. Rein lesend — verändert nichts.

## Abhängigkeiten

- `dto_models::ProjectState` load (vorhanden)
- `git::read_committed_file` — `git2`-basiert (aus Build-Plan)
- `project::diff::detect_changes`, `build_photo_index`, `PageChange` — Änderungserkennung (aus Build-Plan)

**Keine neuen Crates.**

## Wiederverwendung aus `build`

`status` nutzt **dieselbe Änderungserkennung** wie `build`:

- `git::read_committed_file` → committed YAML laden
- `project::diff::detect_changes` → `PageChange` pro Seite (NeedsRebuild / SwapOnly / Clean)
- `project::diff::build_photo_index` → Photo-Lookup für Ratio/Swap-Gruppen

Keine eigene Diff-Logik in `status` — alles lebt in `project/diff.rs`.

---

## Projektzustände

| Zustand | Bedeutung |
| ------- | --------- |
| `empty` | Fotos vorhanden, noch nie gebaut (layout leer) |
| `clean` | Layout existiert, nichts geändert seit letztem Build |
| `modified` | Layout existiert, YAML seit letztem Build geändert |

---

## Kompakte Ansicht: `fotobuch status`

```rust
pub struct StatusReport {
    pub state: ProjectState_,  // empty / clean / modified
    pub total_photos: usize,
    pub group_count: usize,
    pub unplaced: usize,
    pub page_count: usize,
    pub avg_photos_per_page: f64,
    /// Pro Seite: welche Art von Änderung
    pub page_changes: Vec<(usize, PageChange)>,  // (1-basiert, Change)
    /// Detaillierte Seiteninfo (nur für Detail-View)
    pub detail: Option<PageDetail>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectState_ {
    Empty,
    Clean,
    Modified,
}
```

Die CLI-Schicht formatiert das zu:

```text
85 photos in 6 groups (5 unplaced)

Layout: 12 pages, 7.1 photos/page avg
  4 pages modified since last build
    pages 2, 5: need rebuild (ratio mismatch in swapped photos)
    pages 3, 8: compatible swaps only (no rebuild needed)
```

## Detail-Ansicht: `fotobuch status <page>`

```rust
pub struct PageDetail {
    pub page: usize,
    pub photo_count: usize,
    pub change: PageChange,
    pub slots: Vec<SlotInfo>,
}

pub struct SlotInfo {
    pub photo_id: String,
    pub ratio: f64,
    pub swap_group: char,  // A, B, C, ... — on-the-fly berechnet
    pub slot_mm: Option<(f64, f64, f64, f64)>,  // x, y, w, h — None wenn keine Slots
}
```

**Swap-Gruppen**: Fotos mit kompatiblem Ratio (≤5% Abweichung) bekommen denselben Buchstaben. Berechnung on-the-fly via Union-Find oder einfacher: sortiert nach Ratio, gleiche Gruppe wenn Differenz zum Vorgänger ≤5%.

```rust
/// Berechnet Swap-Gruppen: Fotos mit kompatiblem Ratio (≤5%) erhalten denselben Buchstaben.
fn assign_swap_groups(ratios: &[f64]) -> Vec<char> {
    if ratios.is_empty() { return vec![]; }

    // Indizes nach Ratio sortieren
    let mut indices: Vec<usize> = (0..ratios.len()).collect();
    indices.sort_by(|&a, &b| ratios[a].partial_cmp(&ratios[b]).unwrap());

    let mut groups = vec![' '; ratios.len()];
    let mut current_group = b'A';
    groups[indices[0]] = current_group as char;

    for window in indices.windows(2) {
        let prev_ratio = ratios[window[0]];
        let curr_ratio = ratios[window[1]];
        if !ratios_compatible(prev_ratio, curr_ratio) {
            current_group += 1;
        }
        groups[window[1]] = current_group as char;
    }

    groups
}
```

---

## Konsistenzprüfungen

```rust
/// Prüft Konsistenz zwischen photos und layout.
fn check_consistency(state: &ProjectState) -> Vec<String> {
    let photo_index = build_photo_index(state);
    let placed_ids: HashSet<&str> = state.layout.iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();
    let all_ids: HashSet<&str> = photo_index.keys().copied().collect();

    let mut warnings = Vec::new();

    // Orphaned: in layout aber nicht in photos
    let orphaned: Vec<&str> = placed_ids.difference(&all_ids).copied().collect();
    for id in &orphaned {
        // Seite finden
        for page in &state.layout {
            if page.photos.iter().any(|p| p == id) {
                warnings.push(format!(
                    "Orphaned placement: {} on page {} (not in photos)",
                    id, page.page
                ));
            }
        }
    }

    // Ratio-Mismatch nach Swap (nur wenn committed state verfügbar)
    // → wird bereits durch detect_changes abgedeckt, hier nicht duplizieren

    warnings
}
```

Unplaced-Count wird direkt berechnet (kein Warning, nur Info):

```rust
fn count_unplaced(state: &ProjectState) -> usize {
    let placed_ids: HashSet<&str> = state.layout.iter()
        .flat_map(|p| p.photos.iter().map(String::as_str))
        .collect();
    state.photos.iter()
        .flat_map(|g| &g.files)
        .filter(|f| !placed_ids.contains(f.id.as_str()))
        .count()
}
```

---

## Signaturen

### `src/commands/status.rs`

```rust
use crate::dto_models::ProjectState;
use crate::project::diff::{self, PageChange, build_photo_index, ratios_compatible};
use std::collections::HashSet;
use std::path::Path;

pub fn status(project_root: &Path, page: Option<usize>) -> Result<StatusReport> {
    let state = ProjectState::load(&project_root.join("fotobuch.yaml"))?;

    // Basiszahlen
    let total_photos = state.photos.iter().map(|g| g.files.len()).sum();
    let group_count = state.photos.len();
    let unplaced = count_unplaced(&state);
    let page_count = state.layout.len();
    let avg = if page_count > 0 {
        total_photos as f64 / page_count as f64
    } else { 0.0 };

    // Projektzustand bestimmen
    let (project_state, page_changes) = if state.layout.is_empty() {
        (ProjectState_::Empty, vec![])
    } else {
        match git::read_committed_file(project_root, "fotobuch.yaml")? {
            Some(bytes) => {
                let committed: ProjectState = serde_yaml::from_slice(&bytes)?;
                let diff = diff::detect_changes(
                    &state.layout, &committed.layout, &state, &committed
                );
                let changes: Vec<(usize, PageChange)> = diff.pages.iter()
                    .enumerate()
                    .filter(|(_, c)| **c != PageChange::Clean)
                    .map(|(i, c)| (i + 1, c.clone()))
                    .collect();
                let ps = if changes.is_empty() {
                    ProjectState_::Clean
                } else {
                    ProjectState_::Modified
                };
                (ps, changes)
            }
            None => {
                // Kein Commit vorhanden → kann nicht diffzen
                (ProjectState_::Clean, vec![])
            }
        }
    };

    // Konsistenzprüfungen
    let warnings = check_consistency(&state);

    // Detail-View
    let detail = page.map(|p| build_page_detail(&state, p)).transpose()?;

    Ok(StatusReport {
        state: project_state,
        total_photos,
        group_count,
        unplaced,
        page_count,
        avg_photos_per_page: avg,
        page_changes,
        detail,
        warnings,
    })
}

/// Baut Detail-Info für eine einzelne Seite.
fn build_page_detail(state: &ProjectState, page_num: usize) -> Result<PageDetail> {
    if page_num == 0 || page_num > state.layout.len() {
        anyhow::bail!("Invalid page {} (layout has {} pages)", page_num, state.layout.len());
    }

    let page = &state.layout[page_num - 1];
    let photo_index = build_photo_index(state);

    // Ratios sammeln
    let ratios: Vec<f64> = page.photos.iter()
        .map(|id| photo_index.get(id.as_str())
            .map(|(pf, _)| pf.aspect_ratio())
            .unwrap_or(1.0))
        .collect();

    let swap_groups = assign_swap_groups(&ratios);

    let slots: Vec<SlotInfo> = page.photos.iter()
        .enumerate()
        .map(|(i, id)| {
            let slot_mm = page.slots.get(i).map(|s| (s.x_mm, s.y_mm, s.width_mm, s.height_mm));
            SlotInfo {
                photo_id: id.clone(),
                ratio: ratios[i],
                swap_group: swap_groups[i],
                slot_mm,
            }
        })
        .collect();

    Ok(PageDetail {
        page: page_num,
        photo_count: page.photos.len(),
        change: PageChange::Clean, // TODO: aus diff-Ergebnis übernehmen wenn verfügbar
        slots,
    })
}
```

---

## Verhalten ohne Git

Wenn `git::read_committed_file` `None` zurückgibt (kein Commit, kein Repo), funktioniert `status` trotzdem:

- Keine Änderungserkennung → `ProjectState_::Clean` (Annahme)
- Konsistenzprüfungen funktionieren normal
- Detail-View funktioniert normal

---

## Implementierungsreihenfolge

Setzt voraus, dass Build-Plan Schritte 3-4 (git2, project/diff) abgeschlossen sind.

| #   | Schritt | Abhängig von |
| --- | ------- | ------------ |
| 1 | `count_unplaced`, `check_consistency` | — |
| 2 | `assign_swap_groups` | — |
| 3 | `build_page_detail` (Detail-View) | 1, 2 |
| 4 | `status()` Hauptfunktion mit Diff-Integration | 1, 3, Build-Plan Schritt 4 |

Jeder Schritt = ein Commit.

## Tests

| Test | Prüft |
| ---- | ----- |
| Leeres Layout → `ProjectState_::Empty` | Zustandserkennung |
| Nichts geändert → `ProjectState_::Clean`, leere page_changes | Idempotenz |
| Foto getauscht (anderes Ratio) → `Modified`, NeedsRebuild | Diff-Integration |
| Foto getauscht (gleiches Ratio) → `Modified`, SwapOnly | Ratio-Toleranz |
| Unplaced korrekt gezählt | count_unplaced |
| Orphaned Placement → Warning | check_consistency |
| Swap-Gruppen: 3 Fotos Ratio 0.67, 2 Fotos Ratio 1.5 → 2 Gruppen | assign_swap_groups |
| Detail-View: ungültige Seite → Fehler | Validierung |
| Ohne Git-Commit → Status funktioniert (ohne Diff) | Fallback |
