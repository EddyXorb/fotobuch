# Solver Refactoring Plan: solver.rs als Single Entry Point

**Datum**: 8. März 2026
**Status**: In Implementierung

## Zielsetzung

Refactoring der Solver-Architektur, sodass `solver.rs` der einzige Einstiegspunkt ist. Die darunter liegenden Solver (page_layout_solver, book_layout_solver) merken nichts von DTOs und arbeiten weiterhin mit internen data_models.

## Kernänderungen

### 1. Photo Struct erweitern
- **Neu**: `id: String` Feld hinzufügen
- **Änderung**: `Photo::new()` bekommt `id` Parameter
- **Begründung**: Mapping zwischen internen Photos und DTO PhotoFiles
- **Performance**: ✅ Kein Problem - String ist 24 Bytes, vernachlässigbar

### 2. Canvas Struct vereinfachen
- **Entfernen**: `bleed` Feld aus Canvas
- **Neu**: `Canvas::from_book_config()` Konverter-Methode
- **Bleed-Logik**:
  ```rust
  if margin_mm == 0.0:
      canvas_size = page_size + 2*bleed
  else:
      canvas_size = page_size - 2*margin
  ```
- **Begründung**: Bleed ist nur für Canvas-Berechnung relevant, nicht für Layout-Algorithmus

### 3. API-Struktur

```rust
// Simple Switch-Enum ohne Daten
pub enum RequestType {
    SinglePage,   // Ruft page_layout_solver auf
    MultiPage,    // Ruft book_layout_solver auf
}

// Request mit allen Daten
pub struct Request<'a> {
    pub request_type: RequestType,
    pub groups: &'a [PhotoGroup],  // Immer PhotoGroups!
    pub config: &'a BookLayoutSolverConfig,
    pub ga_config: &'a GaConfig,
    pub book_config: &'a BookConfig,
}
```

**Vereinheitlichung**: Beide Request-Typen bekommen `&[PhotoGroup]` für konsistente Gruppen-Info.

### 4. PhotoPlacement bleibt unverändert
- Verwendet weiterhin `photo_idx: u16`
- Mapping photo_idx → photo_id erfolgt bei Konvertierung zu DTO
- **Begründung**: Keine Änderungen an bestehenden Solvern nötig

## Implementierungs-Schritte

### Schritt 1: Photo Struct erweitern

```rust
// src/solver/data_models/photo.rs
pub struct Photo {
    pub id: String,  // NEU
    pub aspect_ratio: f64,
    pub area_weight: f64,
    pub group: String,
    pub timestamp: Option<DateTime<Utc>>,
    pub dimensions: Option<(u32, u32)>,
}

impl Photo {
    pub fn new(id: String, aspect_ratio: f64, area_weight: f64, group: String) -> Self {
        // ...
    }
}
```

**Konverter-Methoden**:
```rust
impl Photo {
    /// Konvertiert PhotoFile zu Photo mit explizitem Gruppen-Namen
    pub fn from_photo_file(file: &PhotoFile, group: &str) -> Self {
        Self {
            id: file.id.clone(),
            aspect_ratio: file.aspect_ratio(),
            area_weight: file.area_weight,
            group: group.to_string(),
            timestamp: Some(file.timestamp),
            dimensions: Some((file.width_px, file.height_px)),
        }
    }
    
    /// Konvertiert PhotoGroups zu Vec<Photo>
    pub fn from_photo_groups(groups: &[PhotoGroup]) -> Vec<Photo> {
        groups
            .iter()
            .flat_map(|group| {
                group.files
                    .iter()
                    .map(|file| Self::from_photo_file(file, &group.group))
            })
            .collect()
    }
}
```

**Tests**:
```rust
#[cfg(test)]
mod converter_tests {
    #[test]
    fn test_from_photo_file() { /* ... */ }
    
    #[test]
    fn test_from_photo_groups() { /* ... */ }
    
    #[test]
    fn test_from_photo_groups_empty() { /* ... */ }
    
    #[test]
    fn test_from_photo_groups_multiple() { /* ... */ }
}
```

### Schritt 2: Canvas vereinfachen

```rust
// src/solver/data_models/canvas.rs
#[derive(Debug, Clone, Copy)]
pub struct Canvas {
    pub width: f64,
    pub height: f64,
    pub beta: f64,
    // bleed entfernt
}

impl Canvas {
    pub fn new(width: f64, height: f64, beta: f64) -> Self {
        assert!(width > 0.0, "Canvas width must be positive");
        assert!(height > 0.0, "Canvas height must be positive");
        assert!(beta >= 0.0, "Beta must be non-negative");
        
        Self { width, height, beta }
    }
    
    /// Erstellt Canvas aus BookConfig mit Bleed/Margin-Logik
    pub fn from_book_config(config: &BookConfig) -> Self {
        let width = if config.margin_mm == 0.0 {
            config.page_width_mm + 2.0 * config.bleed_mm
        } else {
            config.page_width_mm - 2.0 * config.margin_mm
        };
        
        let height = if config.margin_mm == 0.0 {
            config.page_height_mm + 2.0 * config.bleed_mm
        } else {
            config.page_height_mm - 2.0 * config.margin_mm
        };
        
        Self::new(width, height, config.gap_mm)
    }
}
```

**Tests**:
```rust
#[cfg(test)]
mod converter_tests {
    #[test]
    fn test_from_book_config_with_margin() { /* ... */ }
    
    #[test]
    fn test_from_book_config_without_margin() { /* ... */ }
    
    #[test]
    fn test_from_book_config_zero_bleed() { /* ... */ }
}
```

**Anpassungen**:
- Alle Canvas::new() Aufrufe: 4 → 3 Parameter
- Default-Implementation anpassen

### Schritt 3: PageLayout → SolverPageLayout umbenennen

**Umbenennung zur Vermeidung von Verwechslungen**:
- `PageLayout` (solver-intern) → `SolverPageLayout`
- `LayoutPage` (DTO) bleibt unverändert

**Betroffene Dateien**:
- `src/solver/data_models/layout.rs`
- `src/solver/page_layout_solver.rs`
- `src/solver/book_layout_solver.rs`
- Alle Tests

### Schritt 4: SolverPageLayout Konverter

```rust
// src/solver/data_models/layout.rs
impl SolverPageLayout {
    /// Konvertiert SolverPageLayout zu DTO LayoutPage
    pub fn to_layout_page(&self, photos: &[Photo], page_num: usize) -> LayoutPage {
        // Photos und Slots parallel sortieren nach aspect ratio
        let mut pairs: Vec<_> = self.placements
            .iter()
            .map(|placement| {
                let photo = &photos[placement.photo_idx as usize];
                let photo_id = photo.id.clone();
                let slot = Slot {
                    x_mm: placement.x,
                    y_mm: placement.y,
                    width_mm: placement.w,
                    height_mm: placement.h,
                };
                (photo_id, slot, photo.aspect_ratio)
            })
            .collect();
        
        // Sortiere nach aspect ratio (siehe LayoutPage Kommentar)
        pairs.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
        
        let photos: Vec<String> = pairs.iter().map(|(id, _, _)| id.clone()).collect();
        let slots: Vec<Slot> = pairs.iter().map(|(_, slot, _)| slot.clone()).collect();
        
        LayoutPage {
            page: page_num,
            photos,
            slots,
        }
    }
}
```

**Tests**:
```rust
#[cfg(test)]
mod converter_tests {
    #[test]
    fn test_to_layout_page_single() { /* ... */ }
    
    #[test]
    fn test_to_layout_page_sorting() { /* ... */ }
    
    #[test]
    fn test_to_layout_page_empty() { /* ... */ }
}
```

### Schritt 5: solver.rs Hauptlogik

**Struktur: DRY - Code-Duplikation vermeiden**

```rust
// src/solver/solver.rs
use anyhow::Result;
use thiserror::Error;

pub enum RequestType {
    SinglePage,
    MultiPage,
}

pub struct Request<'a> {
    pub request_type: RequestType,
    pub groups: &'a [PhotoGroup],
    pub config: &'a BookLayoutSolverConfig,
    pub ga_config: &'a GaConfig,
    pub book_config: &'a BookConfig,
}

#[derive(Debug, Error)]
pub enum SolverError {
    #[error("Book layout solver failed: {0}")]
    BookLayoutFailed(#[from] book_layout_solver::SolverError),
    
    #[error("Empty input")]
    EmptyInput,
}

pub fn run_solver(request: &Request) -> Result<Vec<LayoutPage>, SolverError> {
    // Gemeinsame Validierung
    if request.groups.is_empty() {
        return Ok(vec![]);
    }
    
    // Gemeinsame Konvertierung: PhotoGroups → Photos
    let photos = Photo::from_photo_groups(request.groups);
    
    if photos.is_empty() {
        return Ok(vec![]);
    }
    
    // Gemeinsame Canvas-Erstellung
    let canvas = Canvas::from_book_config(request.book_config);
    
    // Dispatch based on request type
    match request.request_type {
        RequestType::SinglePage => {
            run_single_page(&photos, &canvas, request)
        }
        RequestType::MultiPage => {
            run_multi_page(&photos, &canvas, request)
        }
    }
}

fn run_single_page(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request,
) -> Result<Vec<LayoutPage>, SolverError> {
    // Run single-page GA solver
    let ga_result = page_layout_solver::run_ga(
        photos,
        canvas,
        request.ga_config
    );
    
    // Convert to DTO
    let layout_page = ga_result.layout.to_layout_page(photos, 1);
    
    Ok(vec![layout_page])
}

fn run_multi_page(
    photos: &[Photo],
    canvas: &Canvas,
    request: &Request,
) -> Result<Vec<LayoutPage>, SolverError> {
    // Run book layout solver (MIP + local search)
    let book_layout = book_layout_solver::solve_book_layout(
        photos,
        request.config,
        canvas,
        request.ga_config
    )?;
    
    // Convert each page to DTO
    let layout_pages: Vec<LayoutPage> = book_layout
        .pages()
        .iter()
        .enumerate()
        .map(|(i, page)| page.to_layout_page(photos, i + 1))
        .collect();
    
    Ok(layout_pages)
}
```

**Code-Duplikation eliminiert**:
- Validierung nur einmal in `run_solver()`
- Konvertierung nur einmal in `run_solver()`
- Canvas-Erstellung nur einmal in `run_solver()`
- Subfunktionen bekommen bereits konvertierte Daten

### Schritt 6: Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    
    fn create_test_photo_file(id: &str, width: u32, height: u32) -> PhotoFile {
        PhotoFile {
            id: id.to_string(),
            source: format!("test/{}.jpg", id),
            width_px: width,
            height_px: height,
            area_weight: 1.0,
            timestamp: Utc::now(),
            hash: None,
        }
    }
    
    fn create_test_book_config() -> BookConfig {
        BookConfig {
            title: "Test".to_string(),
            page_width_mm: 297.0,
            page_height_mm: 210.0,
            bleed_mm: 3.0,
            margin_mm: 10.0,
            gap_mm: 5.0,
            bleed_threshold_mm: 3.0,
        }
    }
    
    #[test]
    fn test_single_page_empty() {
        let request = Request {
            request_type: RequestType::SinglePage,
            groups: &[],
            config: &BookLayoutSolverConfig::default(),
            ga_config: &GaConfig::default(),
            book_config: &create_test_book_config(),
        };
        
        let result = run_solver(&request).unwrap();
        assert_eq!(result.len(), 0);
    }
    
    #[test]
    fn test_single_page_one_group() {
        let group = PhotoGroup {
            group: "vacation".to_string(),
            sort_key: "2024-01-01".to_string(),
            files: vec![
                create_test_photo_file("p1", 1500, 1000),
                create_test_photo_file("p2", 1000, 1500),
            ],
        };
        
        let request = Request {
            request_type: RequestType::SinglePage,
            groups: &[group],
            config: &BookLayoutSolverConfig::default(),
            ga_config: &GaConfig::default(),
            book_config: &create_test_book_config(),
        };
        
        let result = run_solver(&request).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].page, 1);
        assert_eq!(result[0].photos.len(), 2);
    }
    
    #[test]
    fn test_multi_page_empty() {
        let request = Request {
            request_type: RequestType::MultiPage,
            groups: &[],
            config: &BookLayoutSolverConfig::default(),
            ga_config: &GaConfig::default(),
            book_config: &create_test_book_config(),
        };
        
        let result = run_solver(&request).unwrap();
        assert_eq!(result.len(), 0);
    }
    
    #[test]
    fn test_multi_page_multiple_groups() {
        let groups = vec![
            PhotoGroup {
                group: "group1".to_string(),
                sort_key: "2024-01-01".to_string(),
                files: vec![
                    create_test_photo_file("p1", 1500, 1000),
                    create_test_photo_file("p2", 1500, 1000),
                    create_test_photo_file("p3", 1500, 1000),
                ],
            },
            PhotoGroup {
                group: "group2".to_string(),
                sort_key: "2024-01-02".to_string(),
                files: vec![
                    create_test_photo_file("p4", 1000, 1500),
                    create_test_photo_file("p5", 1000, 1500),
                ],
            },
        ];
        
        let request = Request {
            request_type: RequestType::MultiPage,
            groups: &groups,
            config: &BookLayoutSolverConfig::default(),
            ga_config: &GaConfig::default(),
            book_config: &create_test_book_config(),
        };
        
        let result = run_solver(&request).unwrap();
        assert!(result.len() >= 1);
    }
}
```

## Aktualisierungs-Checkliste

### data_models/photo.rs
- [ ] `id: String` Feld hinzufügen
- [ ] `Photo::new()` mit `id` Parameter
- [ ] `Photo::from_photo_file()` implementieren
- [ ] `Photo::from_photo_groups()` implementieren
- [ ] Tests für Konverter schreiben
- [ ] Alle bestehenden Tests aktualisieren (new() Aufrufe)

### data_models/canvas.rs
- [ ] `bleed` Feld entfernen
- [ ] `Canvas::new()` auf 3 Parameter reduzieren
- [ ] `Canvas::from_book_config()` implementieren
- [ ] Tests für Konverter schreiben
- [ ] Default-Implementation anpassen
- [ ] Alle bestehenden Tests aktualisieren

### data_models/layout.rs
- [ ] `PageLayout` → `SolverPageLayout` umbenennen
- [ ] `SolverPageLayout::to_layout_page()` implementieren
- [ ] Tests für Konverter schreiben
- [ ] Alle bestehenden Tests aktualisieren

### page_layout_solver.rs
- [ ] `PageLayout` → `SolverPageLayout` Updates
- [ ] Tests anpassen

### book_layout_solver.rs
- [ ] `PageLayout` → `SolverPageLayout` Updates
- [ ] `BookLayout` Return-Typ behalten (intern)
- [ ] Tests anpassen

### solver.rs
- [ ] `RequestType` enum implementieren
- [ ] `Request` struct implementieren
- [ ] `SolverError` enum implementieren
- [ ] `run_solver()` implementieren
- [ ] `run_single_page()` implementieren
- [ ] `run_multi_page()` implementieren
- [ ] Umfangreiche Tests schreiben

### test_fixtures.rs
- [ ] Helper-Funktionen für Photo mit id aktualisieren

## Verification Steps

1. **Kompilierung**: `cargo build` erfolgreich
2. **Tests**: `cargo test --lib` alle Tests bestehen
3. **Clippy**: `cargo clippy` keine Warnings
4. **Integration**: Commands (build, rebuild) verwenden neue API korrekt

## Design-Entscheidungen (Rationale)

### Warum id in Photo?
- Kein Performance-Problem (24 Bytes)
- Vereinfacht Mapping zwischen DTOs und internen Models
- Alternative (separates Array) wäre komplizierter und fehleranfälliger

### Warum bleed aus Canvas entfernen?
- Bleed ist nur für Canvas-Größenberechnung relevant
- Layout-Algorithmus braucht nur finale Canvas-Größe
- Vereinfacht Canvas struct

### Warum PhotoPlacement unverändert lassen?
- Vermeidet Änderungen an existierenden Solvern
- Index-basiert ist effizienter als String-basiert
- Mapping erfolgt nur bei Konvertierung (einmalig)

### Warum beide RequestTypes mit PhotoGroups?
- Konsistente API
- Gruppen-Info immer verfügbar
- Klare Semantik

### Warum Code-Duplikation eliminieren?
- DRY Prinzip
- Einfachere Wartung
- Weniger Fehlerquellen
- run_solver() macht gemeinsame Arbeit, Subfunktionen nur spezifisches

## Zukünftige Erweiterungen

- Weitere RequestTypes hinzufügen (z.B. `Rebuild`, `Preview`)
- Caching von Konvertierungen
- Async/Parallel Processing bei MultiPage
- Progress Reporting

---

**Ende des Plans**
