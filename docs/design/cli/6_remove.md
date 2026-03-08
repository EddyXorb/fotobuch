# Implementation Plan: `fotobuch remove`

Stand: 2026-03-08

## Überblick

Entfernt Fotos oder ganze Gruppen aus dem Projekt. Pflegt `photos` und `layout` konsistent. Leere Seiten werden automatisch entfernt. Betroffene Seiten brauchen danach Rebuild.

## Abhängigkeiten

- `dto_models::ProjectState` load/save (vorhanden)
- `git::commit_if_changed` — `git2`-basiert (aus Build-Plan)
- `regex` — bereits in Cargo.toml

**Keine neuen Crates.** Insbesondere kein `glob`-Crate — Pattern-Matching erfolgt via Regex auf `photo.source` (konsistent mit `place --filter`).

## Symmetrie

```text
add      <->  remove              (Projekt-Ebene: photos + layout)
place    <->  remove --keep-files (Layout-Ebene: nur layout[].photos)
```

---

## Pattern-Matching

Jedes Pattern ist eine **Regex auf `photo.source`** (absoluter Pfad des Originals). Mehrere Patterns werden mit OR verknüpft.

Zusätzlich: wenn ein Pattern **exakt** einem Gruppennamen entspricht (`state.photos.iter().any(|g| g.group == pattern)`), werden **alle Fotos** dieser Gruppe gematcht — unabhängig vom Source-Pfad.

```rust
/// Sammelt alle Photo-IDs die mindestens einem Pattern entsprechen.
fn match_photos(
    state: &ProjectState,
    patterns: &[String],
) -> Result<MatchResult> {
    let mut matched_ids: HashSet<String> = HashSet::new();
    let mut matched_groups: Vec<String> = Vec::new();

    for pattern in patterns {
        // 1. Exakter Gruppenname?
        if let Some(group) = state.photos.iter().find(|g| g.group == *pattern) {
            for file in &group.files {
                matched_ids.insert(file.id.clone());
            }
            matched_groups.push(group.group.clone());
            continue;
        }

        // 2. Regex auf photo.source
        let re = Regex::new(pattern)
            .context(format!("Invalid pattern: {pattern}"))?;
        for group in &state.photos {
            for file in &group.files {
                if re.is_match(&file.source) {
                    matched_ids.insert(file.id.clone());
                }
            }
        }
    }

    Ok(MatchResult { matched_ids, matched_groups })
}

struct MatchResult {
    matched_ids: HashSet<String>,
    matched_groups: Vec<String>,
}
```

**Beispiele:**

```text
fotobuch remove "2024-01-15_Urlaub"              → Gruppenname-Match
fotobuch remove "IMG_001\.jpg$"                   → Regex auf source
fotobuch remove "Urlaub/IMG_00[1-3]"              → Regex mit Character-Class
fotobuch remove "Urlaub" "Geburtstag"             → Mehrere Patterns (OR)
fotobuch remove --keep-files "IMG_005\.jpg$"      → Nur aus Layout entfernen
```

---

## Ablauf

### Default (ohne `--keep-files`)

1. **YAML laden**
2. **Pattern-Matching**: `match_photos()` → `matched_ids`, `matched_groups`
3. Falls nichts gematcht → `RemoveResult { photos_removed: 0, .. }`, kein Commit
4. **Aus `layout` entfernen**: Fotos und korrespondierende Slots entfernen
5. **Leere Seiten entfernen** + renumbern
6. **Aus `photos` entfernen**: Files aus Gruppen filtern, leere Gruppen entfernen
7. **YAML speichern**, **Git commit**

### Mit `--keep-files`

1-3. wie oben
4. **Nur aus `layout` entfernen** (Fotos + Slots)
5. **Leere Seiten entfernen** + renumbern
6. ~~Aus `photos` entfernen~~ — übersprungen, Fotos bleiben als "unplaced"
7. **YAML speichern**, **Git commit**

---

## Signaturen und Strukturen

### `src/commands/remove.rs`

```rust
use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;

use crate::dto_models::{LayoutPage, ProjectState};

/// Konfiguration — unverändert gegenüber bestehendem Stub.
#[derive(Debug, Clone)]
pub struct RemoveConfig {
    pub patterns: Vec<String>,
    pub keep_files: bool,
}

/// Ergebnis — unverändert gegenüber bestehendem Stub.
#[derive(Debug)]
pub struct RemoveResult {
    pub photos_removed: usize,
    pub placements_removed: usize,
    pub groups_removed: Vec<String>,
    pub pages_affected: Vec<usize>,  // 1-basiert, vor Renumbering
}

pub fn remove(project_root: &Path, config: &RemoveConfig) -> Result<RemoveResult> {
    let mut state = ProjectState::load(&project_root.join("fotobuch.yaml"))?;

    // 1. Pattern-Matching
    let matches = match_photos(&state, &config.patterns)?;
    if matches.matched_ids.is_empty() {
        return Ok(RemoveResult {
            photos_removed: 0,
            placements_removed: 0,
            groups_removed: vec![],
            pages_affected: vec![],
        });
    }

    // 2. Aus Layout entfernen (immer, auch bei --keep-files)
    let layout_result = remove_from_layout(&mut state.layout, &matches.matched_ids);

    // 3. Leere Seiten entfernen + renumbern
    let removed_pages = remove_empty_pages(&mut state.layout);
    renumber_pages(&mut state.layout);

    // 4. Aus Photos entfernen (nur ohne --keep-files)
    let mut groups_removed = matches.matched_groups.clone();
    let photos_removed = if config.keep_files {
        0
    } else {
        remove_from_photos(&mut state.photos, &matches.matched_ids, &mut groups_removed)
    };

    // 5. YAML + Git
    state.save(&project_root.join("fotobuch.yaml"))?;

    let commit_msg = if config.keep_files {
        format!("remove: {} placements from layout (photos kept)", layout_result.placements_removed)
    } else {
        format!("remove: {} photos", photos_removed)
    };
    git::commit_if_changed(project_root, &commit_msg)?;

    Ok(RemoveResult {
        photos_removed,
        placements_removed: layout_result.placements_removed,
        groups_removed,
        pages_affected: layout_result.pages_affected,
    })
}
```

#### Aus Layout entfernen

```rust
struct LayoutRemoveResult {
    placements_removed: usize,
    pages_affected: Vec<usize>,  // 1-basiert
}

/// Entfernt gematchte Fotos aus allen Layout-Seiten.
/// Photos und Slots sind index-gekoppelt — beide werden parallel gefiltert.
fn remove_from_layout(
    layout: &mut [LayoutPage],
    matched_ids: &HashSet<String>,
) -> LayoutRemoveResult {
    let mut placements_removed = 0;
    let mut pages_affected = Vec::new();

    for page in layout.iter_mut() {
        let before = page.photos.len();

        // Photos und Slots parallel filtern (index-gekoppelt)
        let keep: Vec<bool> = page.photos.iter()
            .map(|id| !matched_ids.contains(id))
            .collect();

        let new_photos: Vec<String> = page.photos.iter()
            .zip(&keep)
            .filter(|(_, &k)| k)
            .map(|(id, _)| id.clone())
            .collect();

        let new_slots = if page.slots.len() == page.photos.len() {
            // Slots vorhanden und index-gekoppelt
            page.slots.iter()
                .zip(&keep)
                .filter(|(_, &k)| k)
                .map(|(slot, _)| slot.clone())
                .collect()
        } else {
            // Slots leer oder inkonsistent — leeren
            vec![]
        };

        let removed = before - new_photos.len();
        if removed > 0 {
            pages_affected.push(page.page);
            placements_removed += removed;
        }

        page.photos = new_photos;
        page.slots = new_slots;
    }

    LayoutRemoveResult { placements_removed, pages_affected }
}
```

#### Leere Seiten entfernen

```rust
/// Entfernt Seiten ohne Fotos aus dem Layout.
/// Gibt die Nummern der entfernten Seiten zurück (1-basiert, vor Renumbering).
fn remove_empty_pages(layout: &mut Vec<LayoutPage>) -> Vec<usize> {
    let empty_pages: Vec<usize> = layout.iter()
        .filter(|p| p.photos.is_empty())
        .map(|p| p.page)
        .collect();

    layout.retain(|p| !p.photos.is_empty());
    empty_pages
}

/// Nummeriert alle LayoutPage.page Felder sequenziell (1-basiert).
/// Identisch mit rebuild::renumber_pages — könnte nach shared.rs verschoben werden.
fn renumber_pages(layout: &mut [LayoutPage]) {
    for (i, page) in layout.iter_mut().enumerate() {
        page.page = i + 1;
    }
}
```

#### Aus Photos entfernen

```rust
/// Entfernt gematchte Fotos aus state.photos.
/// Leere Gruppen werden komplett entfernt.
/// Gibt die Anzahl entfernter Fotos zurück.
fn remove_from_photos(
    photos: &mut Vec<PhotoGroup>,
    matched_ids: &HashSet<String>,
    groups_removed: &mut Vec<String>,
) -> usize {
    let mut total_removed = 0;

    for group in photos.iter_mut() {
        let before = group.files.len();
        group.files.retain(|f| !matched_ids.contains(&f.id));
        total_removed += before - group.files.len();
    }

    // Leere Gruppen entfernen
    let empty_groups: Vec<String> = photos.iter()
        .filter(|g| g.files.is_empty())
        .map(|g| g.group.clone())
        .collect();

    for g in &empty_groups {
        if !groups_removed.contains(g) {
            groups_removed.push(g.clone());
        }
    }

    photos.retain(|g| !g.files.is_empty());
    total_removed
}
```

---

## Hinweis: Slots nach Remove

Nach dem Entfernen sind die verbleibenden Slots geometrisch **veraltet** — das Layout war für eine andere Fotozahl optimiert. Die Seite braucht einen Rebuild (`fotobuch build` oder `fotobuch rebuild`).

Die alten Slots werden trotzdem beibehalten (nicht geleert), weil:

- Das Preview-PDF bleibt halbwegs brauchbar (Fotos an ungefähr richtigen Positionen)
- `status` kann die Seite als "needs rebuild" markieren
- Der User sieht sofort das Ergebnis, auch ohne Rebuild

---

## Shared: `renumber_pages`

`renumber_pages` wird auch von `rebuild` (Range mit flex) benötigt. Beim Implementieren nach `commands/shared.rs` verschieben:

```rust
// commands/shared.rs (ergänzen)
pub fn renumber_pages(layout: &mut [LayoutPage]) {
    for (i, page) in layout.iter_mut().enumerate() {
        page.page = i + 1;
    }
}
```

---

## Implementierungsreihenfolge

Setzt voraus, dass Build-Plan Schritt 3 (git2) abgeschlossen ist.

| #   | Schritt | Abhängig von |
| --- | ------- | ------------ |
| 1 | `match_photos` (Gruppenname + Regex auf source) | — |
| 2 | `remove_from_layout` (photos + slots parallel filtern) | 1 |
| 3 | `remove_empty_pages`, `renumber_pages` | 2 |
| 4 | `remove_from_photos` (files + leere Gruppen entfernen) | 1 |
| 5 | `remove()` Hauptfunktion mit `--keep-files` Logik + Git | 2, 3, 4 |

Jeder Schritt = ein Commit. Tests vor jedem Commit.

## Konventionen

- **Conventional Commits**: z.B. `feat: implement pattern matching for remove`, `feat: implement slot-coupled removal from layout`
- **Tests**: Unit-Tests + Integrationstests für jeden Schritt
- **`mod solver` unberührt**: `remove` ruft keinen Solver auf — nur YAML-Manipulation

## Tests

| Test | Prüft |
| ---- | ----- |
| Einzelnes Foto entfernen → aus photos und layout weg, Slots angepasst | remove_from_layout + remove_from_photos |
| Ganze Gruppe per Name entfernen → alle Fotos + Layout-Einträge weg | Gruppenname-Match |
| Regex-Pattern: `"IMG_00[1-3]"` matcht genau 3 Fotos | Regex auf source |
| Ungültiges Regex-Pattern → sinnvoller Fehler | Fehlerbehandlung |
| Mehrere Patterns (OR-Verknüpfung) | Patterns werden vereinigt |
| `--keep-files`: nur Layout bereinigt, photos bleibt | keep_files Flag |
| `--keep-files`: Fotos sind danach "unplaced" (in photos, nicht in layout) | Symmetrie mit place |
| Slots werden parallel zu photos gefiltert (Index-Kopplung) | remove_from_layout |
| Seite wird leer → automatisch entfernt, Renumbering korrekt | remove_empty_pages |
| Nichts gematcht → 0 entfernt, kein Git-Commit | Idempotenz |
| Leere Gruppe nach Foto-Entfernung → Gruppe entfernt | remove_from_photos |
| Mehrere Seiten betroffen → alle in pages_affected | Vollständigkeit |
