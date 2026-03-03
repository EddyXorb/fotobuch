# 3. Rust-Module Details

## Übersicht

Die Rust-Codebase ist in 6 Module aufgeteilt, jedes mit klarer Verantwortung.

```
src/
├── main.rs          → CLI & Workflow-Orchestrierung
├── models.rs        → Datenstrukturen
├── scanner.rs       → Foto-Scanning & EXIF
├── python_env.rs    → Python Environment Management
├── solver.rs        → Solver-Orchestration
└── typst_export.rs  → Typst & PDF-Export
```

---

## `main.rs` - CLI Entry Point

**Verantwortung:**
- CLI-Argument-Parsing mit `clap`
- Workflow-Orchestrierung
- Error-Handling & User-Feedback

**Workflow:**
```rust
fn main() -> Result<()> {
    // 1. Parse CLI args
    let args = Args::parse();
    
    // 2. Ensure Python environment
    let py_env = PythonEnv::ensure()?;
    
    // 3. Scan photos
    let groups = scanner::scan_photo_dirs(&args.input)?;
    
    // 4. Solve layout
    let pages = solver::solve(&groups, &config, args.solver_mode)?;
    
    // 5. Export to PDF
    typst_export::generate_pdf(&pages, &args.output)?;
    
    Ok(())
}
```

---

## `models.rs` - Datenstrukturen

**Kerntypen:**

```rust
/// Ein Foto mit Metadaten
pub struct Photo {
    pub id: String,                          // Eindeutige ID (für API)
    pub path: PathBuf,                       // Dateipfad (NUR intern)
    pub timestamp: Option<NaiveDateTime>,    // EXIF oder Ordner-Timestamp
    pub dimensions: Option<(u32, u32)>,      // (width, height) in Pixeln
    pub area_weight: f64,                    // Relative Flächenzuweisung (default: 1.0)
    pub group: String,                       // Gruppenname (Ordnername)
}

/// Gruppe von Fotos (z.B. aus einem Ordner)
pub struct PhotoGroup {
    pub label: String,
    pub timestamp: Option<NaiveDateTime>,
    pub photos: Vec<Photo>,
}

/// Eine Seite im Fotobuch
pub struct Page {
    pub placements: Vec<Placement>,
}

/// Platzierung eines Fotos auf einer Seite
pub struct Placement {
    pub photo: Photo,
    pub x_mm: f64,
    pub y_mm: f64,
    pub width_mm: f64,
    pub height_mm: f64,
}

/// Konfiguration für das Fotobuch
pub struct BookConfig {
    pub page_width_mm: f64,
    pub page_height_mm: f64,
    pub margin_mm: f64,
    pub gap_mm: f64,
    pub max_photos_per_page: usize,
    pub target_pages: Option<usize>,
    pub max_aspect_deviation: f64,  // Default: 0.2 (20%)
    pub timeout_seconds: u32,       // Default: 30
    
    // Objective Weights (alle via CLI anpassbar)
    pub weight_aspect_ratio: f64,   // Default: 1.0
    pub weight_area: f64,           // Default: 10.0 (starke Bestrafung)
    pub weight_group_cohesion: f64, // Default: 2.0
    pub weight_page_count: f64,     // Default: 0.5
}

/// DTO für API-Kommunikation (OHNE path!)
#[derive(Serialize)]
pub struct PhotoApiDto {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub timestamp: Option<NaiveDateTime>,
    pub group: String,
    pub area_weight: f64,
}

impl From<&Photo> for PhotoApiDto {
    fn from(photo: &Photo) -> Self {
        Self {
            id: photo.id.clone(),
            width: photo.dimensions.map(|(w, _)| w).unwrap_or(1920),
            height: photo.dimensions.map(|(_, h)| h).unwrap_or(1080),
            timestamp: photo.timestamp,
            group: photo.group.clone(),
            area_weight: photo.area_weight,
        }
    }
}
```

---

## `scanner.rs` - Foto-Scanning

**Verantwortung:**
- Ordner rekursiv durchsuchen
- Zeitstempel aus Ordnernamen parsen
- EXIF-Daten auslesen
- Photo-IDs generieren
- **Gruppen lexikalisch sortieren (nach Gruppennamen)**

**Hauptfunktion:**
```rust
pub fn scan_photo_dirs(base_dir: &Path) -> Result<Vec<PhotoGroup>> {
    let mut groups = Vec::new();
    
    for entry in fs::read_dir(base_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let group = scan_group(&entry.path())?;
            groups.push(group);
        }
    }
    
    // Lexikalisch nach Gruppenname sortieren
    groups.sort_by(|a, b| a.label.cmp(&b.label));
    
    Ok(groups)
}

fn scan_group(dir: &Path) -> Result<PhotoGroup> {
    let label = dir.file_name().unwrap().to_string_lossy().to_string();
    let timestamp = parse_timestamp_from_name(&label);
    
    let mut photos = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if is_image(&path) {
            let photo = scan_photo(&path, &label)?;
            photos.push(photo);
        }
    }
    
    Ok(PhotoGroup { label, timestamp, photos })
}

fn scan_photo(path: &Path, group: &str) -> Result<Photo> {
    // ID generieren (z.B. relative path als hash)
    let id = generate_photo_id(path);
    
    // EXIF auslesen
    let exif = read_exif(path)?;
    let timestamp = exif.timestamp();
    let dimensions = exif.dimensions();
    
    Ok(Photo {
        id,
        path: path.to_path_buf(),
        timestamp,
        dimensions,
        area_weight: 1.0,  // Default
        group: group.to_string(),
    })
}
```

**Zeitstempel-Parsing:**
- `2024-07-15_Urlaub` → 2024-07-15 00:00:00
- `20240715_Ferien` → 2024-07-15 00:00:00
- `2024-07-15_18-30-00` → 2024-07-15 18:30:00

---

## `python_env.rs` - Python Environment Management

**Verantwortung:**
- Prüfen ob `uv` und `python3` installiert sind
- `.venv/` erstellen falls nicht vorhanden
- Dependencies aus eingebackenem `uv.lock` installieren
- Installation verifizieren

**Hauptstruktur:**
```rust
pub struct PythonEnv {
    venv_path: PathBuf,
    python_exe: PathBuf,
}

impl PythonEnv {
    /// Stellt Python-Umgebung sicher, gibt Fehler bei fehlenden Prerequisites
    pub fn ensure() -> Result<Self> {
        Self::check_uv_installed()?;
        Self::check_python_installed()?;
        
        let venv_path = PathBuf::from(".venv");
        
        if !venv_path.exists() {
            Self::create_venv_from_lock(&venv_path)?;
        }
        
        let python_exe = Self::python_executable(&venv_path);
        Self::verify_installation(&python_exe)?;
        
        Ok(Self { venv_path, python_exe })
    }
    
    fn check_uv_installed() -> Result<()> {
        Command::new("uv").arg("--version").output()
            .context("uv not found. Install: curl -LsSf https://astral.sh/uv/install.sh | sh")?;
        Ok(())
    }
    
    fn check_python_installed() -> Result<()> {
        let output = Command::new("python3").arg("--version").output()
            .context("python3 not found")?;
        
        // Version prüfen (≥3.10)
        // ...
        
        Ok(())
    }
    
    fn create_venv_from_lock(venv_path: &Path) -> Result<()> {
        // Eingebackenes uv.lock schreiben
        const UV_LOCK: &str = env!("UV_LOCK_CONTENT");
        fs::write("uv.lock", UV_LOCK)?;
        
        // uv sync --frozen
        Command::new("uv")
            .args(["sync", "--frozen"])
            .status()
            .context("uv sync failed")?;
        
        Ok(())
    }
    
    fn verify_installation(python_exe: &Path) -> Result<()> {
        let output = Command::new(python_exe)
            .args(["-c", "import ortools; print(ortools.__version__)"])
            .output()?;
        
        if !output.status.success() {
            bail!("OR-Tools not properly installed");
        }
        
        Ok(())
    }
    
    pub fn python_path(&self) -> &Path {
        &self.python_exe
    }
}
```

---

## `solver.rs` - Solver-Orchestration

**Verantwortung:**
- Solver-Modus-Selection (Heuristic/CLI/API)
- JSON-Serialisierung für Python
- Photo-ID → Photo Mapping
- Page-Rekonstruktion aus API-Response

**Hauptfunktionen:**
```rust
pub enum SolverMode {
    Cli,        // Python subprocess (default)
    Api,        // REST API (schneller, Server muss bereits laufen)
}

pub fn solve(
    groups: &[PhotoGroup],
    config: &BookConfig,
    mode: SolverMode,
    session_id: &str,
    debug: bool
) -> Result<Vec<Page>> {
    match mode {
        SolverMode::Cli => solve_ortools_cli(groups, config, session_id, debug),
        SolverMode::Api => solve_ortools_api(groups, config, session_id, debug),
    }
}

fn solve_ortools_cli(
    groups: &[PhotoGroup], 
    config: &BookConfig,
    session_id: &str,
    debug: bool
) -> Result<Vec<Page>> {
    let py_env = PythonEnv::ensure()?;
    
    log::info!("Using CLI mode with session_id: {}", session_id);
    
    // Photo-Map erstellen (ID → Photo)
    let mut photo_map = HashMap::new();
    let api_photos: Vec<PhotoApiDto> = groups
        .iter()
        .flat_map(|g| &g.photos)
        .map(|p| {
            photo_map.insert(p.id.clone(), p.clone());
            PhotoApiDto::from(p)
        })
        .collect();
    
    // JSON erstellen (OHNE paths!)
    let solver_input = json!({
        "photos": api_photos,
        "config": config,
    });
    
    // Python über stdin aufrufen mit Session-ID
    let input_json = serde_json::to_string(&solver_input)?;
    
    let mut cmd = Command::new(py_env.python_path());
    cmd.args(["-m", "photosolver", "solve", "-"])
        .env("PHOTOSOLVER_SESSION_ID", session_id)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    if debug {
        cmd.env("PHOTOSOLVER_DEBUG", "1");
    }
    
    let mut child = cmd.spawn()?;
    child.stdin.as_mut().unwrap().write_all(input_json.as_bytes())?;
    let output = child.wait_with_output()?;
    
    if !output.status.success() {
        log::error!("Python solver failed: {}", String::from_utf8_lossy(&output.stderr));
        bail!("Solver failed");
    }
    
    log::debug!("Solver output: {} bytes", output.stdout.len());
    
    // Pages rekonstruieren (mit paths aus photo_map)
    let solver_output: SolverOutput = serde_json::from_slice(&output.stdout)?;
    reconstruct_pages(solver_output.pages, &photo_map)
}

fn reconstruct_pages(
    api_pages: Vec<ApiPage>,
    photo_map: &HashMap<String, Photo>
) -> Result<Vec<Page>> {
    api_pages
        .into_iter()
        .map(|api_page| {
            let placements = api_page.placements
                .into_iter()
                .map(|p| {
                    let photo = photo_map.get(&p.photo_id)
                        .ok_or_else(|| anyhow!("Photo ID not found: {}", p.photo_id))?
                        .clone();
                    
                    Ok(Placement {
                        photo,
                        x_mm: p.x_mm,
                        y_mm: p.y_mm,
                        width_mm: p.width_mm,
                        height_mm: p.height_mm,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            
            Ok(Page { placements })
        })
        .collect()
}

fn solve_ortools_api(
    groups: &[PhotoGroup], 
    config: &BookConfig,
    session_id: &str,
    debug: bool
) -> Result<Vec<Page>> {
    let url = env::var("PHOTOSOLVER_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:8000".to_string());
    
    log::info!("Using API mode {} with session_id: {}", url, session_id);
    
    // WICHTIG: Kein Health-Check, kein Auto-Start
    // Annahme: Server läuft bereits (User-Verantwortung)
    // Bei Fehler: Klare Error-Message mit Start-Anleitung
    
    // Photo-Map + API-DTO erstellen (wie in CLI-Mode)
    // ...
    
    // POST /solve mit Session-ID Header
    let response = reqwest::blocking::Client::new()
        .post(&format!("{}/solve", url))
        .header("X-Session-ID", session_id)
        .header("X-Debug", if debug { "1" } else { "0" })
        .json(&api_input)
        .timeout(Duration::from_secs(120))  // Generous
        .send()
        .map_err(|e| anyhow!(
            "API not reachable. Start server with:\n  cd python && uv run photosolver serve\n\nError: {}", 
            e
        ))?;
    
    // ... response handling
}
```

---

## `typst_export.rs` - Typst & PDF-Export

**Verantwortung:**
- `.typ`-Source generieren
- Mit Typst-API zu PDF kompilieren

**Unverändert zur bisherigen Implementation:**
```rust
pub fn generate_pdf(pages: &[Page], output_path: &Path) -> Result<()> {
    let typ_source = generate_typ_source(pages);
    
    // Optional: .typ-Datei schreiben
    let typ_path = output_path.with_extension("typ");
    fs::write(&typ_path, &typ_source)?;
    
    // Typst kompilieren
    compile_typst_to_pdf(&typ_source, output_path)?;
    
    Ok(())
}
```

---

## Dependencies

```toml
# Cargo.toml
[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
chrono = "0.4"
kamadak-exif = "0.6"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "5"
reqwest = { version = "0.11", features = ["blocking", "json"] }
typst = "0.14"
typst-pdf = "0.14"

[build-dependencies]
# Für uv.lock einbacken
```

---

## Nächstes Dokument

➡️ [4. Python-Package](4_python-package.md) - Detaillierte Python-Modul-Beschreibungen
