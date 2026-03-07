# Implementation Plan: `fotobuch add`

Stand: 2026-03-07

## Überblick

Der `add` Command scannt Verzeichnisse nach Bilddateien, gruppiert sie, liest EXIF-Metadaten, erkennt Duplikate und fügt sie zum Projekt hinzu.

## 1. Funktionale Anforderungen

### 1.1 Input
- **Argumente**: Liste von Pfaden (Verzeichnisse oder einzelne Dateien)
- **Flag**: `--allow-duplicates` (optional, default: false)

### 1.2 Output
- **AddResult**: Zusammenfassung mit hinzugefügten Gruppen, übersprungenen Dateien, Warnungen
- **Side Effects**: 
  - Aktualisierte `fotobuch.yaml` (photos-Sektion erweitert)
  - Git Commit: `add: N photos in M groups`

### 1.3 Gruppierungslogik

**Regel**: Jedes Verzeichnis, das **direkt** Bilddateien enthält, wird eine Gruppe.

**Beispiel**:
```
~/Fotos/Urlaub/
├── Tag1/
│   ├── IMG_001.jpg  ← Tag1-Gruppe
│   └── IMG_002.jpg
├── Tag2/
│   └── IMG_003.jpg  ← Tag2-Gruppe
├── Abend/           ← kein Foto direkt hier
│   └── Kneipe/
│       └── IMG_010.jpg  ← Kneipe-Gruppe
└── panorama.jpg     ← Urlaub-Gruppe (root)
```

**Gruppenname**: Relativer Pfad ab dem `add`-Argument
- `fotobuch add ~/Fotos/Urlaub/` → Gruppen: `Urlaub`, `Urlaub/Tag1`, `Urlaub/Tag2`, `Urlaub/Abend/Kneipe`

**Einzeldateien**: 
- `fotobuch add ~/Fotos/portrait.jpg` → Gruppe: `Fotos` (Elternverzeichnis)
- Mehrere Einzeldateien aus gleichem Ordner → eine Gruppe

### 1.4 Zeitstempel-Heuristik

**Pro Gruppe** wird ein `sort_key` (ISO 8601 Timestamp) bestimmt:

1. **Ordnername parsen**: `2024-01-15_Urlaub` → `2024-01-15T00:00:00`
   - Regex: `^\d{4}-\d{2}-\d{2}` am Anfang des Namens
2. **Frühestes EXIF-Datum**: `DateTimeOriginal` aller Fotos in der Gruppe
3. **Früheste File mtime**: Falls kein EXIF vorhanden

**Erste verfügbare Quelle gewinnt.** Wird nur beim ersten `add` einer Gruppe gesetzt, danach manuell änderbar.

### 1.5 Duplikaterkennung

**Methode**: Partieller Hash (schnell, praktisch zuverlässig)
- Erste 64 KB + Letzte 64 KB + Dateigröße
- Hash mit SHA-256 oder Blake3

**Verhalten**:

| Situation | Erkennung | Aktion |
|---|---|---|
| Selber absoluter Pfad bereits in YAML | Pfad-Vergleich | Überspringen, `skipped += 1` |
| Selber Hash, anderer Pfad | Hash-Kollision | Warnung + Überspringen (außer `--allow-duplicates`) |
| Gruppe existiert bereits | Gruppenname-Check | Fotos zur existierenden Gruppe hinzufügen |

**Ausgaben**:
```
Warning: IMG_001.jpg (from ~/Backup/) has identical content to Urlaub/IMG_001.jpg
Use --allow-duplicates to add anyway.
```

### 1.6 YAML-Update

**Zu ergänzen**: `photos`-Sektion

```yaml
photos:
  - group: "2024-01-15_Urlaub"
    sort_key: "2024-01-15T09:23:00"
    files:
      - id: "2024-01-15_Urlaub/IMG_001.jpg"
        source: "/home/user/Fotos/2024-01-15_Urlaub/IMG_001.jpg"
        width_px: 6000
        height_px: 4000
        area_weight: 1.0
```

**ID-Generierung**:
- Default: `<group>/<filename>` (z.B. `Urlaub/IMG_001.jpg`)
- Bei Clash mit existierender ID: Suffix `_1`, `_2`, ... anhängen
- ID muss projekt-weit eindeutig sein

**area_weight**: Default `1.0` (gleichgewichtet)

---

## 2. Architektur

### 2.1 Module-Struktur

```
src/
├── commands/
│   └── add.rs           # Orchestrierung (bestehendes Stub)
├── input/
│   ├── scanner.rs       # EXISTIERT: scan_photo_dirs()
│   └── metadata.rs      # NEU: EXIF-Extraktion, Hashing
└── project/             # NEU: YAML-Operationen
    ├── mod.rs
    ├── state.rs         # ProjectState load/save
    └── photo_group.rs   # Gruppierungs-Logik
```

### 2.2 Datenfluss

```
1. add(project_root, config)
   ↓
2. ProjectState::load("fotobuch.yaml")
   ↓
3. scan_and_group(config.paths)
   ├→ Verzeichnisse rekursiv scannen
   ├→ Bilddateien filtern (.jpg, .jpeg, .png, ...)
   └→ Nach Verzeichnis gruppieren
   ↓
4. extract_metadata(photos)
   ├→ EXIF lesen (dimensions, DateTimeOriginal)
   ├→ Partiellen Hash berechnen
   └→ Timestamp-Heuristik pro Gruppe
   ↓
5. deduplicate(scanned, existing)
   ├→ Pfad-Check
   ├→ Hash-Check
   └→ Warnungen sammeln
   ↓
6. merge_groups(new_groups, project_state.photos)
   ├→ Existierende Gruppen erweitern
   ├→ Neue Gruppen hinzufügen
   └→ IDs generieren (Clash-Behandlung)
   ↓
7. ProjectState::save("fotobuch.yaml")
   ↓
8. git_commit("add: N photos in M groups")
   ↓
9. Return AddResult
```

---

## 3. Detaillierte Implementierung

### 3.1 Neue Structs (models/)

**project/state.rs**:
```rust
use serde::{Deserialize, Serialize};
use crate::models::ProjectConfig;

/// Complete project state as persisted in fotobuch.yaml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectState {
    pub config: ProjectConfig,
    pub photos: Vec<PhotoGroup>,
    pub layout: Vec<LayoutPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoGroup {
    pub group: String,
    pub sort_key: String,  // ISO 8601
    pub files: Vec<PhotoFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoFile {
    pub id: String,
    pub source: String,  // Absoluter Pfad
    pub width_px: u32,
    pub height_px: u32,
    #[serde(default = "default_area_weight")]
    pub area_weight: f64,
}

fn default_area_weight() -> f64 { 1.0 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutPage {
    pub page: usize,  // 1-based, nur Info
    pub photos: Vec<String>,  // Photo IDs
    pub slots: Vec<Slot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slot {
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

impl ProjectState {
    pub fn load(path: &Path) -> Result<Self>;
    pub fn save(&self, path: &Path) -> Result<()>;
}
```

### 3.2 Metadata-Extraktion (input/metadata.rs)

**Dependencies**:
- `kamadak-exif` (für EXIF-Parsing)
- `blake3` (für schnelles Hashing)

```rust
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use anyhow::Result;

pub struct PhotoMetadata {
    pub width_px: u32,
    pub height_px: u32,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub hash: String,  // Blake3 hex
}

pub fn extract_metadata(path: &Path) -> Result<PhotoMetadata> {
    // 1. EXIF lesen für dimensions + timestamp
    let file = File::open(path)?;
    let mut bufreader = std::io::BufReader::new(file);
    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut bufreader).ok();
    
    // Dimensions aus EXIF oder Image crate
    let (width_px, height_px) = extract_dimensions(&exif, path)?;
    
    // Timestamp aus EXIF DateTimeOriginal
    let timestamp = extract_timestamp(&exif);
    
    // 2. Partieller Hash
    let hash = compute_partial_hash(path)?;
    
    Ok(PhotoMetadata { width_px, height_px, timestamp, hash })
}

fn compute_partial_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();
    
    let mut hasher = blake3::Hasher::new();
    
    // Erste 64 KB
    let mut buffer = vec![0u8; 65536];
    let n = file.read(&mut buffer)?;
    hasher.update(&buffer[..n]);
    
    // Letzte 64 KB (falls Datei groß genug)
    if file_size > 65536 {
        file.seek(SeekFrom::End(-65536))?;
        let n = file.read(&mut buffer)?;
        hasher.update(&buffer[..n]);
    }
    
    // Dateigröße mit einbeziehen
    hasher.update(&file_size.to_le_bytes());
    
    Ok(hasher.finalize().to_hex().to_string())
}
```

### 3.3 Timestamp-Heuristik (project/timestamp.rs)

```rust
use chrono::{DateTime, NaiveDate, Utc};
use std::path::Path;

pub fn determine_group_timestamp(
    group_name: &str,
    photos: &[PhotoMetadata],
) -> String {
    // 1. Ordnername parsen
    if let Some(ts) = parse_date_from_name(group_name) {
        return ts.to_rfc3339();
    }
    
    // 2. Frühestes EXIF-Datum
    let exif_dates: Vec<_> = photos.iter()
        .filter_map(|p| p.timestamp)
        .collect();
    if let Some(&earliest) = exif_dates.iter().min() {
        return earliest.to_rfc3339();
    }
    
    // 3. File mtime als Fallback
    // (wird beim Scannen ermittelt)
    Utc::now().to_rfc3339()  // Fallback
}

fn parse_date_from_name(name: &str) -> Option<DateTime<Utc>> {
    use regex::Regex;
    let re = Regex::new(r"^(\d{4})-(\d{2})-(\d{2})").ok()?;
    let caps = re.captures(name)?;
    
    let year: i32 = caps[1].parse().ok()?;
    let month: u32 = caps[2].parse().ok()?;
    let day: u32 = caps[3].parse().ok()?;
    
    NaiveDate::from_ymd_opt(year, month, day)?
        .and_hms_opt(0, 0, 0)?
        .and_local_timezone(Utc)
        .single()
}
```

### 3.4 Gruppierung (input/grouper.rs)

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct ScannedGroup {
    pub name: String,
    pub photos: Vec<ScannedPhoto>,
}

pub struct ScannedPhoto {
    pub original_path: PathBuf,
    pub metadata: PhotoMetadata,
}

pub fn scan_and_group(paths: &[PathBuf]) -> Result<Vec<ScannedGroup>> {
    let mut groups: HashMap<String, Vec<ScannedPhoto>> = HashMap::new();
    
    for path in paths {
        if path.is_dir() {
            scan_directory(path, path, &mut groups)?;
        } else if path.is_file() && is_image_file(path) {
            // Einzeldatei: Gruppe = Elternverzeichnis
            let group_name = path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("photos")
                .to_string();
            
            let metadata = extract_metadata(path)?;
            groups.entry(group_name).or_default().push(ScannedPhoto {
                original_path: path.clone(),
                metadata,
            });
        }
    }
    
    Ok(groups.into_iter()
        .map(|(name, photos)| ScannedGroup { name, photos })
        .collect())
}

fn scan_directory(
    root: &Path,
    current: &Path,
    groups: &mut HashMap<String, Vec<ScannedPhoto>>,
) -> Result<()> {
    let mut has_direct_images = false;
    
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Rekursiv weiterscannen
            scan_directory(root, &path, groups)?;
        } else if is_image_file(&path) {
            has_direct_images = true;
        }
    }
    
    // Wenn dieses Verzeichnis direkt Fotos enthält → Gruppe erstellen
    if has_direct_images {
        let group_name = current.strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        
        let group_name = if group_name.is_empty() {
            root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("photos")
                .to_string()
        } else {
            group_name
        };
        
        let photos: Vec<_> = std::fs::read_dir(current)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file() && is_image_file(&e.path()))
            .map(|e| {
                let metadata = extract_metadata(&e.path())?;
                Ok(ScannedPhoto {
                    original_path: e.path(),
                    metadata,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        
        groups.insert(group_name, photos);
    }
    
    Ok(())
}

fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| matches!(e.to_lowercase().as_str(), "jpg" | "jpeg" | "png"))
        .unwrap_or(false)
}
```

### 3.5 Duplikaterkennung (commands/add.rs)

```rust
fn deduplicate(
    scanned: Vec<ScannedGroup>,
    existing: &[PhotoGroup],
    allow_duplicates: bool,
) -> (Vec<ScannedGroup>, Vec<String>, usize) {
    let mut warnings = Vec::new();
    let mut skipped = 0;
    
    // Hash-Set aller existierenden Fotos
    let existing_paths: HashSet<_> = existing.iter()
        .flat_map(|g| &g.files)
        .map(|f| f.source.as_str())
        .collect();
    
    let existing_hashes: HashMap<_, _> = existing.iter()
        .flat_map(|g| &g.files)
        .map(|f| (f.hash.clone(), f.source.as_str()))
        .collect();
    
    let filtered: Vec<ScannedGroup> = scanned.into_iter()
        .map(|mut group| {
            group.photos.retain(|photo| {
                let path_str = photo.original_path.to_string_lossy();
                
                // Check 1: Selber Pfad
                if existing_paths.contains(path_str.as_ref()) {
                    skipped += 1;
                    return false;
                }
                
                // Check 2: Hash-Kollision
                if let Some(&existing_path) = existing_hashes.get(&photo.metadata.hash) {
                    if !allow_duplicates {
                        warnings.push(format!(
                            "Duplicate: {} has identical content to {}",
                            path_str, existing_path
                        ));
                        skipped += 1;
                        return false;
                    }
                }
                
                true
            });
            group
        })
        .filter(|g| !g.photos.is_empty())
        .collect();
    
    (filtered, warnings, skipped)
}
```

### 3.6 ID-Generierung und Merge

```rust
fn merge_groups(
    scanned: Vec<ScannedGroup>,
    project_state: &mut ProjectState,
) -> Result<Vec<GroupSummary>> {
    let mut summaries = Vec::new();
    
    for scanned_group in scanned {
        // Prüfen ob Gruppe schon existiert
        let existing_group = project_state.photos.iter_mut()
            .find(|g| g.group == scanned_group.name);
        
        let timestamp = determine_group_timestamp(
            &scanned_group.name,
            &scanned_group.photos.iter()
                .map(|p| &p.metadata)
                .collect::<Vec<_>>(),
        );
        
        let files: Vec<PhotoFile> = scanned_group.photos.iter()
            .map(|photo| {
                let id = generate_unique_id(
                    &scanned_group.name,
                    &photo.original_path,
                    project_state,
                );
                
                PhotoFile {
                    id,
                    source: photo.original_path.to_string_lossy().to_string(),
                    width_px: photo.metadata.width_px,
                    height_px: photo.metadata.height_px,
                    area_weight: 1.0,
                }
            })
            .collect();
        
        if let Some(group) = existing_group {
            // Zur existierenden Gruppe hinzufügen
            let count = files.len();
            group.files.extend(files);
            summaries.push(GroupSummary {
                name: scanned_group.name,
                photo_count: count,
                timestamp,
            });
        } else {
            // Neue Gruppe erstellen
            let count = files.len();
            project_state.photos.push(PhotoGroup {
                group: scanned_group.name.clone(),
                sort_key: timestamp.clone(),
                files,
            });
            summaries.push(GroupSummary {
                name: scanned_group.name,
                photo_count: count,
                timestamp,
            });
        }
    }
    
    Ok(summaries)
}

fn generate_unique_id(
    group_name: &str,
    photo_path: &Path,
    project_state: &ProjectState,
) -> String {
    let filename = photo_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    
    let base_id = format!("{}/{}", group_name, filename);
    
    // Prüfen auf Clash
    let existing_ids: HashSet<_> = project_state.photos.iter()
        .flat_map(|g| &g.files)
        .map(|f| f.id.as_str())
        .collect();
    
    let mut id = base_id.clone();
    let mut counter = 1;
    
    while existing_ids.contains(id.as_str()) {
        id = format!("{}_{}", base_id, counter);
        counter += 1;
    }
    
    id
}
```

### 3.7 Git Integration (project/git.rs)

```rust
use std::process::Command;
use std::path::Path;
use anyhow::{Context, Result};

pub fn commit(project_root: &Path, message: &str) -> Result<()> {
    // git add fotobuch.yaml
    Command::new("git")
        .current_dir(project_root)
        .args(&["add", "fotobuch.yaml"])
        .output()
        .context("Failed to git add")?;
    
    // Prüfen ob es was zu committen gibt
    let status = Command::new("git")
        .current_dir(project_root)
        .args(&["diff", "--cached", "--quiet"])
        .status()
        .context("Failed to check git diff")?;
    
    if status.success() {
        // Nichts zu committen
        return Ok(());
    }
    
    // git commit -m "$message"
    Command::new("git")
        .current_dir(project_root)
        .args(&["commit", "-m", message])
        .output()
        .context("Failed to git commit")?;
    
    Ok(())
}
```

### 3.8 Hauptfunktion (commands/add.rs)

```rust
pub fn add(project_root: &Path, config: &AddConfig) -> Result<AddResult> {
    // 1. Projekt-Zustand laden
    let yaml_path = project_root.join("fotobuch.yaml");
    let mut project_state = ProjectState::load(&yaml_path)?;
    
    // 2. Verzeichnisse scannen und gruppieren
    let scanned = scan_and_group(&config.paths)?;
    
    // 3. Duplikate erkennen
    let (filtered, warnings, skipped) = deduplicate(
        scanned,
        &project_state.photos,
        config.allow_duplicates,
    );
    
    // 4. Gruppen mergen
    let summaries = merge_groups(filtered, &mut project_state)?;
    
    // 5. YAML speichern
    project_state.save(&yaml_path)?;
    
    // 6. Git Commit
    let total_photos: usize = summaries.iter().map(|s| s.photo_count).sum();
    let commit_msg = format!("add: {} photos in {} groups", total_photos, summaries.len());
    project::git::commit(project_root, &commit_msg)?;
    
    // 7. Ergebnis zurückgeben
    Ok(AddResult {
        groups_added: summaries,
        skipped,
        warnings,
    })
}
```

---

## 4. Dependencies

**Cargo.toml hinzufügen**:

```toml
[dependencies]
# Bereits vorhanden:
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_yaml = "0.9"

# Neu benötigt:
kamadak-exif = "0.5"      # EXIF-Parsing
blake3 = "1.5"            # Schnelles Hashing
chrono = { version = "0.4", features = ["serde"] }  # Timestamps
regex = "1.10"            # Datum-Parsing aus Ordnernamen
```

---

## 5. Testing-Strategie

### 5.1 Unit Tests

**metadata.rs**:
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_partial_hash_consistency() {
        // Hash sollte identisch sein für selbe Datei
    }
    
    #[test]
    fn test_exif_extraction() {
        // Mit Test-Foto inkl. EXIF
    }
}
```

**timestamp.rs**:
```rust
#[test]
fn test_parse_date_from_name() {
    assert_eq!(
        parse_date_from_name("2024-01-15_Urlaub"),
        Some(...)
    );
}
```

**grouper.rs**:
```rust
#[test]
fn test_directory_grouping() {
    // Mit tempdir Test-Struktur aufbauen
}
```

### 5.2 Integration Tests

```rust
// tests/integration_add.rs
use tempfile::TempDir;

#[test]
fn test_add_photos_end_to_end() {
    // 1. Projekt erstellen
    // 2. Test-Fotos vorbereiten
    // 3. add() aufrufen
    // 4. YAML prüfen
    // 5. Git-Commit prüfen
}

#[test]
fn test_add_duplicate_detection() {
    // 1. Fotos einmal adden
    // 2. Nochmal adden → sollte skippen
    // 3. Mit --allow-duplicates → sollte durchgehen
}
```

---

## 6. CLI-Integration

**cli.rs erweitern**:

```rust
#[derive(Parser)]
enum Cli {
    New { /* ... */ },
    
    /// Add photos to the project
    Add {
        /// Directories or individual files to add
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        
        /// Allow adding files with identical content
        #[arg(long)]
        allow_duplicates: bool,
    },
}

// In run():
Cli::Add { paths, allow_duplicates } => {
    let config = AddConfig { paths, allow_duplicates };
    let project_root = std::env::current_dir()?;
    
    let result = commands::add(&project_root, &config)?;
    
    // Ausgabe formatieren
    for group in &result.groups_added {
        println!("Added group \"{}\" ({} photos, {})",
                 group.name, group.photo_count, group.timestamp);
    }
    
    if result.skipped > 0 {
        println!("Skipped {} files (already exist)", result.skipped);
    }
    
    for warning in &result.warnings {
        eprintln!("Warning: {}", warning);
    }
    
    Ok(())
}
```

---

## 7. Implementierungs-Reihenfolge

### Phase 1: Datenstrukturen (1 Commit)
1. `src/project/mod.rs`, `state.rs` - ProjectState, PhotoGroup, etc.
2. `src/models.rs` - public exports
3. Unit-Tests für Serialisierung

### Phase 2: Metadata-Extraktion (1 Commit)
1. `Cargo.toml` - Dependencies hinzufügen
2. `src/input/metadata.rs` - EXIF + Hashing
3. Unit-Tests

### Phase 3: Timestamp-Heuristik (1 Commit)
1. `src/project/timestamp.rs` - parse_date_from_name, determine_group_timestamp
2. Unit-Tests

### Phase 4: Gruppierung (1 Commit)
1. `src/input/grouper.rs` - scan_and_group
2. Unit-Tests mit tempdir

### Phase 5: Add-Command (1 Commit)
1. `src/commands/add.rs` - Hauptlogik implementieren
2. `src/project/git.rs` - Git-Commit-Funktion

### Phase 6: CLI-Integration (1 Commit)
1. `src/cli.rs` - Add-Subcommand
2. Ausgabe-Formatierung

### Phase 7: Integration Tests (1 Commit)
1. `tests/integration_add.rs`
2. End-to-End-Tests

### Phase 8: Manueller Test + Refinement
1. Echte Fotos adden
2. Edge Cases testen
3. Clippy + Cargo build warnings beheben

---

## 8. Offene Fragen / Entscheidungen

1. **EXIF-Library**: kamadak-exif vs. rexiv2?
   - **Empfehlung**: kamadak-exif (pure Rust, keine C-Dependencies)

2. **Hash-Algo**: Blake3 vs. SHA-256?
   - **Empfehlung**: Blake3 (deutlich schneller, kryptografisch sicher genug)

3. **Git via std::process oder git2 crate?**
   - **Empfehlung**: std::process::Command (einfacher, git binary sowieso required)

4. **Wie behandeln wenn git nicht im PATH?**
   - **Empfehlung**: Warnung loggen, Command trotzdem erfolgreich (git optional für add)

5. **Image-Format-Support**: Nur JPG/PNG oder auch RAW?
   - **Empfehlung**: Start mit JPG/PNG, RAW später via separate Feature-Flag

---

## 9. Erwartete Ausgaben

### Erfolgreicher Add:
```
$ fotobuch add ~/Fotos/2024-01-15_Urlaub/ ~/Fotos/2024-02-20_Geburtstag/
Added group "2024-01-15_Urlaub" (47 photos, 2024-01-15T09:23:00)
Added group "2024-02-20_Geburtstag" (23 photos, 2024-02-20T14:00:00)
```

### Mit Duplikaten:
```
$ fotobuch add ~/Backup/
Warning: IMG_001.jpg has identical content to Urlaub/IMG_001.jpg
Warning: IMG_002.jpg has identical content to Urlaub/IMG_002.jpg
Skipped 45 files (duplicates, use --allow-duplicates to override)
Added group "Backup" (2 photos, 2024-01-15T10:00:00)
```

### Gruppe existiert bereits:
```
$ fotobuch add ~/Fotos/2024-01-15_Urlaub/new_subfolder/
Skipped 47 files (already in project)
Added 3 new photos to group "2024-01-15_Urlaub"
```
