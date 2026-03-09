# Implementation Plan: `fotobuch status`

Stand: 2026-03-08

## Überblick

Zeigt Projektstatus: Fotos, Gruppen, Layout-Zusammenfassung, geänderte Seiten seit letztem Build. Rein lesend — verändert nichts.

## Abhängigkeiten

- `StateManager` — öffnet Projekt, lädt State, erkennt Änderungen intern via `StateDiff`
- `build:helpers::build_photo_index` (aus Build-Plan)

**Keine neuen Crates.**

## Wiederverwendung aus `build`

`status` nutzt **dieselbe Änderungserkennung** wie `build`:

- `StateManager::open(project_root)` → Projekt öffnen, committed State intern verwaltet
- `StateManager::has_changes_since_last_build()` → Gesamtstatus (clean / modified)
- `StateManager::modified_pages()` → `Vec<usize>` mit 1-basierten Seitennummern (geändert seit letztem Build)
- `build::helpers::build_photo_index()` → Photo-Lookup für Ratio/Swap-Gruppen (`HashMap<String, (PhotoFile, String)>`)

Der committed State wird intern vom StateManager geladen und verwaltet. `status` hat Zugriff auf `mgr.state` (aktuell) und kann über `StateManager::modified_pages()` die geänderten Seiten direkt abfragen.

Keine eigene Diff-Logik in `status` — Änderungserkennung ist zentral im StateManager implementiert.

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
    pub project_name: String,
    pub state: ProjectState_,  // empty / clean / modified
    pub total_photos: usize,
    pub group_count: usize,
    pub unplaced: usize,
    pub page_count: usize,
    pub avg_photos_per_page: f64,
    /// Seitennummern (1-basiert) die seit letztem Build geändert wurden
    pub page_changes: Vec<usize>,
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
Project: urlaub
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
    pub modified: bool,  // true wenn seit letztem Build geändert
    pub slots: Vec<SlotInfo>,
}

pub struct SlotInfo {
    pub photo_id: String,
    pub ratio: f64,
    pub swap_group: char,  // A, B, C, ... — on-the-fly berechnet
    pub slot_mm: (f64, f64, f64, f64),  // x, y, w, h
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
use crate::commands::build::helpers::build_photo_index;
use crate::state_manager::StateManager;
use crate::dto_models::ProjectState;
use std::collections::HashSet;
use std::path::Path;

pub fn status(project_root: &Path, page: Option<usize>) -> Result<StatusReport> {
    let mgr = StateManager::open(project_root)?;
    let project_name = mgr.project_name().to_owned();

    // Basiszahlen
    let total_photos = mgr.state.photos.iter().map(|g| g.files.len()).sum();
    let group_count = mgr.state.photos.len();
    let unplaced = count_unplaced(&mgr.state);
    let page_count = mgr.state.layout.len();
    let avg = if page_count > 0 {
        total_photos as f64 / page_count as f64
    } else { 0.0 };

    // Projektzustand bestimmen
    // StateManager::has_changes_since_last_build() liefert den Gesamtstatus
    // StateManager::modified_pages() gibt die modizierten Seitennummern (1-basiert) zurück
    let (project_state, page_changes) = if mgr.state.layout.is_empty() {
        (ProjectState_::Empty, vec![])
    } else if !mgr.has_changes_since_last_build() {
        (ProjectState_::Clean, vec![])
    } else {
        // Geänderte Seiten direkt vom StateManager
        let modified = mgr.modified_pages();
        if modified.is_empty() {
            (ProjectState_::Clean, vec![])
        } else {
            (ProjectState_::Modified, modified)  // Vec<usize> (1-basierte Seitennummern)
        }
    };

    // Konsistenzprüfungen
    let warnings = check_consistency(&mgr.state);

    // Detail-View (nur für angeforderte Seite)
    let modified_pages = if mgr.state.layout.is_empty() {
        vec![]
    } else {
        mgr.modified_pages()
    };

    let detail = page.map(|p| build_page_detail(&mgr.state, p, &modified_pages)).transpose()?;

    Ok(StatusReport {
        project_name,
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
fn build_page_detail(state: &ProjectState, page_num: usize, modified_pages: &[usize]) -> Result<PageDetail> {
    if page_num == 0 || page_num > state.layout.len() {
        anyhow::bail!("Invalid page {} (layout has {} pages)", page_num, state.layout.len());
    }

    let page = &state.layout[page_num - 1];
    let photo_index = build_photo_index(&state.photos);

    // Ratios sammeln
    let ratios: Vec<f64> = page.photos.iter()
        .map(|id| photo_index.get(id.as_str())
            .map(|(pf, _)| pf.aspect_ratio())
            .unwrap_or(1.0))
        .collect();

    let swap_groups = assign_swap_groups(&ratios);

    // Prüfe, ob diese Seite seit letztem Build geändert wurde
    let modified = modified_pages.contains(&page_num);

    let slots: Vec<SlotInfo> = page.photos.iter()
        .enumerate()
        .map(|(i, id)| {
            let slot_mm = page.slots.get(i)
                .map(|s| (s.x_mm, s.y_mm, s.width_mm, s.height_mm))
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
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
        modified,
        slots,
    })
}
```

---

## Verhalten ohne Git

Der `StateManager` erkennt intern, ob ein Git-Repository vorhanden ist und ob ein Build-Commit existiert. `status` muss das nicht selbst prüfen:

- Kein Repo / kein Build-Commit → `StateManager::has_changes_since_last_build()` gibt `false`, `modified_pages()` gibt leere Vec
- Folge: keine Änderungserkennung → `ProjectState_::Clean` oder `ProjectState_::Empty`
- Konsistenzprüfungen funktionieren normal (unabhängig von Git)
- Detail-View funktioniert normal

---

## Implementierungsreihenfolge

Setzt voraus, dass `StateManager` implementiert ist.

| #   | Schritt | Abhängig von |
| --- | ------- | ------------ |
| 1 | `count_unplaced`, `check_consistency` | — |
| 2 | `assign_swap_groups` | — |
| 3 | `build_page_detail` (Detail-View) | 1, 2 |
| 4 | `status()` Hauptfunktion mit Diff-Integration | 1, 3, StateManager |

Jeder Schritt = ein Commit.

## Tests

| Test | Prüft |
| ---- | ----- |
| Leeres Layout → `ProjectState_::Empty` | Zustandserkennung |
| Nichts geändert → `ProjectState_::Clean`, leere page_changes | Idempotenz |
| Layout-Foto-Liste geändert → `Modified`, page_changes enthält Seitennummern | Änderungserkennung |
| Unplaced korrekt gezählt | count_unplaced |
| Orphaned Placement → Warning | check_consistency |
| Swap-Gruppen: 3 Fotos Ratio 0.67, 2 Fotos Ratio 1.5 → 2 Gruppen | assign_swap_groups |
| Detail-View: ungültige Seite → Fehler | Validierung |
| Ohne Git-Commit → Status funktioniert (StateManager Fallback) | Fallback |
