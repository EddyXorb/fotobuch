# Implementation Plan: `fotobuch add`

Stand: 2026-03-08

## Überblick

Scannt Verzeichnisse nach Bilddateien, gruppiert sie, liest EXIF-Metadaten, erkennt Duplikate und fügt sie zum Projekt hinzu. Nutzt den StateManager für YAML-Persistenz und Git-Commits.

## CLI-Interface

```text
$ fotobuch add --help
Add photos to the project

Usage: fotobuch add [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Directories or individual files to add

Options:
      --allow-duplicates  Allow adding files with identical content
  -h, --help              Print help
```

## Abhängigkeiten

- `StateManager` — YAML laden/speichern, Git-Commit (aus `state_manager.rs`)
- `input::scanner::scan_photo_dirs` — existierend, rekursives Scanning
- `input::metadata::compute_partial_hash` — existierend, Duplikaterkennung

### Crates (bereits vorhanden)

- `kamadak-exif` — EXIF-Parsing
- `blake3` — Partielles Hashing
- `chrono` — Timestamps
- `regex` — Datum-Parsing aus Ordnernamen

**Keine neuen Crates.**

---

## Gruppierungslogik

Jedes Verzeichnis das **direkt** Bilddateien enthält wird eine Gruppe.

```text
~/Fotos/Urlaub/
├── Tag1/
│   ├── IMG_001.jpg  ← Tag1-Gruppe
│   └── IMG_002.jpg
├── Tag2/
│   └── IMG_003.jpg  ← Tag2-Gruppe
└── panorama.jpg     ← Urlaub-Gruppe (root)
```

**Gruppenname**: Relativer Pfad ab dem `add`-Argument. Bei Einzeldateien: Elternverzeichnis.

## Zeitstempel-Heuristik (sort_key)

Pro Gruppe wird ein `sort_key` (ISO 8601) bestimmt. Erste verfügbare Quelle gewinnt:

1. **Ordnername parsen**: `2024-01-15_Urlaub` → `2024-01-15T00:00:00`
2. **Frühestes EXIF-Datum**: `DateTimeOriginal` aller Fotos der Gruppe
3. **Früheste File mtime**: Falls kein EXIF vorhanden

## Duplikaterkennung

**Methode**: Partieller Hash (erste 64 KB + letzte 64 KB + Dateigröße) via Blake3.

| Situation | Erkennung | Aktion |
|---|---|---|
| Selber absoluter Pfad bereits im YAML | Pfad-Vergleich | Überspringen |
| Selber Hash, anderer Pfad | Hash-Kollision | Warnung + Überspringen (außer `--allow-duplicates`) |
| Gruppe existiert bereits | Gruppenname-Check | Fotos zur existierenden Gruppe hinzufügen |

## ID-Generierung

Format: `<group>_<filename_without_ext>` (analog zu `input/scanner.rs`).

Bei Namenskollision innerhalb der Gruppe: Suffix `_1`, `_2`, ... ab dem zweiten Duplikat.

---

## Ablauf

1. **StateManager öffnen** → committet ggf. manuelle User-Edits
2. **Pfade scannen**: `scan_photo_dirs()` für jedes Argument
3. **Duplikate erkennen**: Pfad-Check + Hash-Check gegen existierende Fotos
4. **Gruppen mergen**: Existierende Gruppen erweitern, neue hinzufügen
5. **Gruppen nach sort_key sortieren**
6. **StateManager finishen** → schreibt YAML + committet: `add: N photos in M groups`

---

## Signaturen und Strukturen

### `src/commands/add.rs`

```rust
use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::dto_models::{PhotoFile, PhotoGroup};
use crate::input::metadata::compute_partial_hash;
use crate::input::scanner::scan_photo_dirs;
use crate::state_manager::StateManager;

#[derive(Debug, Clone)]
pub struct AddConfig {
    pub paths: Vec<PathBuf>,
    pub allow_duplicates: bool,
}

#[derive(Debug)]
pub struct GroupSummary {
    pub name: String,
    pub photo_count: usize,
    pub timestamp: String,
}

#[derive(Debug)]
pub struct AddResult {
    pub groups_added: Vec<GroupSummary>,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> {
    let mut mgr = StateManager::open(project_root)?;

    // 1. Existierende Pfade und Hashes sammeln
    let existing_paths: HashSet<PathBuf> = mgr.state.photos.iter()
        .flat_map(|g| &g.files)
        .map(|f| PathBuf::from(&f.source))
        .collect();
    let existing_hashes: HashSet<String> = mgr.state.photos.iter()
        .flat_map(|g| &g.files)
        .filter_map(|f| f.hash.clone())
        .collect();

    // 2. Scannen
    let mut all_groups = Vec::new();
    for path in &config.paths {
        let groups = scan_photo_dirs(path)?;
        all_groups.extend(groups);
    }

    // 3. Duplikate filtern + Gruppen mergen
    let mut added_groups = Vec::new();
    let mut skipped = 0;
    let mut warnings = Vec::new();

    for mut scanned_group in all_groups {
        let (filtered, skip_count, warns) = deduplicate(
            &mut scanned_group.files,
            &existing_paths,
            &existing_hashes,
            config.allow_duplicates,
        );
        skipped += skip_count;
        warnings.extend(warns);

        if filtered.is_empty() { continue; }

        let photo_count = filtered.len();
        scanned_group.files = filtered;

        // Merge: existierende Gruppe erweitern oder neue hinzufügen
        merge_group(&mut mgr.state.photos, scanned_group.clone());

        added_groups.push(GroupSummary {
            name: scanned_group.group,
            photo_count,
            timestamp: scanned_group.sort_key,
        });
    }

    // 4. Sortieren
    mgr.state.photos.sort_by(|a, b| a.sort_key.cmp(&b.sort_key));

    // 5. Finish
    if !added_groups.is_empty() {
        let total: usize = added_groups.iter().map(|g| g.photo_count).sum();
        mgr.finish(&format!("add: {} photos in {} groups", total, added_groups.len()))?;
    }

    Ok(AddResult { groups_added: added_groups, skipped, warnings })
}
```

#### Duplikatfilter

```rust
/// Filtert Duplikate aus einer Dateiliste.
/// Gibt (gefilterte Files, Skip-Count, Warnungen) zurück.
fn deduplicate(
    files: &mut Vec<PhotoFile>,
    existing_paths: &HashSet<PathBuf>,
    existing_hashes: &HashSet<String>,
    allow_duplicates: bool,
) -> (Vec<PhotoFile>, usize, Vec<String>) {
    let mut kept = Vec::new();
    let mut skipped = 0;
    let mut warnings = Vec::new();

    for mut file in files.drain(..) {
        let path = PathBuf::from(&file.source);

        // Pfad-Check
        if existing_paths.contains(&path) {
            skipped += 1;
            continue;
        }

        // Hash berechnen + Check
        match compute_partial_hash(&path) {
            Ok(hash) => {
                if !allow_duplicates && existing_hashes.contains(&hash) {
                    warnings.push(format!(
                        "Duplicate (by hash): {}", path.display()
                    ));
                    skipped += 1;
                    continue;
                }
                file.hash = Some(hash);
            }
            Err(e) => {
                warnings.push(format!("Hash failed for {}: {}", path.display(), e));
                continue;
            }
        }

        kept.push(file);
    }

    (kept, skipped, warnings)
}
```

#### Gruppen-Merge

```rust
/// Fügt eine gescannte Gruppe in die Projektfotos ein.
/// Existierende Gruppe → Fotos anhängen. Neue Gruppe → hinzufügen.
fn merge_group(photos: &mut Vec<PhotoGroup>, scanned: PhotoGroup) {
    if let Some(existing) = photos.iter_mut().find(|g| g.group == scanned.group) {
        existing.files.extend(scanned.files);
    } else {
        photos.push(scanned);
    }
}
```

---

## Bestehende Module (keine Änderungen nötig)

- `input/scanner.rs` — `scan_photo_dirs()` existiert bereits, liefert `Vec<PhotoGroup>`
- `input/metadata.rs` — `compute_partial_hash()` existiert bereits

## Zu entfernen

- `execute_add()` in `commands/add.rs` — CLI-Formatierung gehört in die CLI-Schicht, nicht ins Command
- `project/git.rs` — Git-Operationen werden vom StateManager übernommen

---

## Implementierungsreihenfolge

Setzt voraus: StateManager ist implementiert.

| # | Schritt                                        | Abhängig von       |
|---|------------------------------------------------|--------------------|
| 1 | `deduplicate` (Pfad + Hash-Check)              | —                  |
| 2 | `merge_group` (existierende Gruppe erweitern)  | —                  |
| 3 | `add()` Hauptfunktion mit StateManager         | 1, 2, StateManager |

Jeder Schritt = ein Commit.

## Konventionen

- **Conventional Commits**: z.B. `feat: implement duplicate detection for add`, `feat: implement add command with StateManager`
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen
- **`clippy --fix`** vor jedem Commit ausführen
- **`cargo build`** regelmäßig, alle Warnings beheben
- **`mod solver` unberührt**: Alle Implementierungen in `src/commands/add.rs`
- **Dateigröße**: Bei >300 Zeilen in Submodule aufteilen
- **Kein `mod.rs`**: Untermodule als gleichnamige Dateien

## Tests

| Test | Prüft |
|------|-------|
| Fotos aus einem Verzeichnis → korrekte Gruppe + Fotos | scan + merge |
| Fotos aus mehreren Verzeichnissen → mehrere Gruppen | Gruppierung |
| Duplikat-Pfad → übersprungen, `skipped += 1` | Pfad-Check |
| Duplikat-Hash → Warnung + übersprungen | Hash-Check |
| `--allow-duplicates` → Hash-Duplikat wird trotzdem hinzugefügt | Flag |
| Existierende Gruppe → Fotos werden angehängt | merge_group |
| ID-Kollision → Suffix `_1`, `_2` | generate_unique_id |
| Gruppen nach sort_key sortiert | Sortierung |
| StateManager::finish() wird mit korrekter Message aufgerufen | Integration |
| Leeres Verzeichnis → keine Gruppen, kein Commit | Edge case |
