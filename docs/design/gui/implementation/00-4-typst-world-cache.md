# Phase 0.4: Persistente TypstWorld mit comemo-Cache

**Ziel**: Typst-Render von ~1 s auf ~50–150 ms für Folge-Compiles bringen,
indem Fonts, File-Slots und comemo-Cache über Compile-Läufe erhalten bleiben —
analog zur Strategie von `typst watch`.

**Voraussetzung**: Keine. Reine Lib-Änderung in `src/output/`, CLI-Pfade
bleiben nutzbar.

---

## Hintergrund

Typst cached über `comemo` (Memoisierung nach Hash der Eingaben) alles, was
teuer ist: Font-Shaping, Image-Decode, Layout. `typst watch` hält genau deshalb
**eine** `SystemWorld`-Instanz über die gesamte Session hinweg und invalidiert
nur geänderte Dateien per Versions-Counter in "File-Slots".

`src/output/typst.rs` baut aktuell in **jedem** `render_pages`-Aufruf eine neue
`SimpleWorld`, scannt Systemfonts neu, liest alle Dateien neu aus dem
Dateisystem. Zusätzlich spawnt `gui/background.rs` pro Seite einen Rayon-Task,
der `render_pages(&[page])` aufruft — **N Compiles für N Seiten**. Beide
Entscheidungen werfen jeden comemo-Cache-Hit weg.

---

## 0.4.1 — Neuer Typ `TypstWorld` in `src/output/typst_world.rs`

`TypstWorld` **ersetzt** die bisherige `SimpleWorld` vollständig (siehe 0.4.3) —
es gibt am Ende nur noch eine `World`-Implementation im Codebase. Kein
`mod.rs` (CLAUDE.md-Regel): `src/output/typst.rs` deklariert
`pub mod typst_world;`, die Datei liegt als `src/output/typst_world.rs`
daneben. Falls `typst.rs` später wächst, kann es zu einem Ordner `typst/`
mit `typst.rs` auf derselben Ebene werden — nicht in dieser Phase.

```rust
pub struct TypstWorld {
    root: PathBuf,
    main_id: FileId,
    library: LazyHash<Library>,        // einmalig, nie invalidiert
    book:    LazyHash<FontBook>,       // einmalig
    fonts:   Vec<FontSlot>,            // einmalig per Fonts::searcher
    slots:   RefCell<HashMap<FileId, FileSlot>>,
    revision: Cell<u32>,
}

struct FileSlot {
    /// Auf Disk zuletzt gesehen:
    mtime: SystemTime,
    len:   u64,
    /// Cached content:
    source: OnceCell<Source>,  // nur .typ
    bytes:  OnceCell<Bytes>,   // alles andere (Bilder etc.)
    /// Letzter Compile-Lauf, in dem diese Slot angefasst wurde.
    /// Für eventuelles Prunen von Zombie-Slots.
    last_accessed: u32,
}
```

Konstruktor `TypstWorld::new(project_root: &Path, project_name: &str) -> Result<Self>`:
- Einmal `Fonts::searcher().search()`.
- `main_id` stabil aus `VirtualPath::within_root`.
- `slots` leer, `revision = 0`.
- **Liest noch keine Dateien** — das passiert beim ersten Compile via `World`-Trait.

### `World`-Trait-Impl

Struktur wie bei der gelöschten `SimpleWorld` (Font-Init, `main_id` aus
`VirtualPath::within_root`), mit einem Unterschied: `source()` und
`file()` konsultieren zuerst die `slots`. Bei Hit → `clone()` der bereits
gespeicherten `Source`/`Bytes`. Bei Miss → von Disk lesen, `OnceCell` füllen,
`mtime`/`len` merken, zurückgeben.

`Bytes` und `Source` haben **inhaltsbasierte Hashes** — d.h. zweimal denselben
Inhalt zurückzugeben ergibt denselben Hash und damit denselben comemo-Key.
Das ist die ganze Magie: wir müssen *keine* eigenen Hashes berechnen, solange
wir denselben `Bytes`-Wert zurückgeben.

---

## 0.4.2 — `reload()`: mtime-basierte Invalidation

```rust
impl TypstWorld {
    /// Vor jedem Compile aufzurufen.
    /// 1. revision += 1
    /// 2. Für jeden Slot prüfen, ob die Datei sich geändert hat.
    /// 3. comemo::evict(EVICT_MAX_AGE) am Ende.
    pub fn reload(&mut self) -> Result<()>;
}
```

Pro Slot: `fs::metadata()` lesen, `(mtime, len)` mit gespeichertem Wert
vergleichen. Unterschied → `OnceCell` leeren (`take()`), sonst unverändert lassen.
Der nächste `source()`/`file()`-Aufruf auf einen geleerten Slot liest neu.

Slots, die in den letzten 10 Compiles nicht angefasst wurden, werden entfernt
(Zombie-Cleanup: Bilder, die aus dem Layout raus sind).

**Konstanten**:
- `EVICT_MAX_AGE: usize = 10` für `comemo::evict(10)`. Passt zur Typst-CLI-Default.

### mtime-Auflösung

Auf ext4 hat `mtime` eine Nanosekunden-Auflösung, auf manchen FS (NTFS, alte
macOS-Versionen) nur 1 s. Wir vergleichen **(mtime, len)** als Tupel — ein
1-Byte-Swap bei identischer Sekunde wird durch `len` nicht abgedeckt, aber in
Kombination ist das für unseren Use Case (Bilder in einem manuell verwalteten
Projekt) robust genug. Als Fallback kann später ein blake3-Hash der ersten
4 KB dazukommen; jetzt nicht.

---

## 0.4.3 — API-Refaktor in `src/output/typst.rs`

**Bestehende `render_pages()` weg, ersetzt durch zwei Primitive**:

```rust
impl TypstWorld {
    /// Ein Compile für das gesamte Dokument. Teurer Teil; comemo-gecached
    /// für alles, was sich nicht geändert hat.
    pub fn compile_document(&self) -> Result<PagedDocument>;
}

/// Rasterisiert eine einzelne Seite aus einem bereits compileten Dokument.
/// Stateless, parallelisierbar, unabhängig von TypstWorld.
pub fn rasterize_page(
    doc: &PagedDocument,
    page: usize,
    pixel_per_pt: f32,
) -> Result<RenderedPage>;
```

Eine kleine Convenience-Funktion oben drauf, damit Aufrufer nicht jedes Mal
zwei Zeilen schreiben müssen:

```rust
pub fn render_pages_with_world(
    world: &mut TypstWorld,
    pages: &[usize],
    pixel_per_pt: f32,
) -> Result<Vec<RenderedPage>> {
    world.reload()?;
    let doc = world.compile_document()?;
    pages.iter().map(|&p| rasterize_page(&doc, p, pixel_per_pt)).collect()
}
```

Parallelisierung passiert **nicht** in der Lib — der Aufrufer entscheidet.
Die GUI wird parallelisieren (siehe 0.4.4), der CLI-Pfad bleibt seriell.

### CLI-Pfad nutzt ebenfalls `TypstWorld` — `SimpleWorld` wird entfernt

Die bisherige `SimpleWorld` in `src/output/typst.rs` wird **komplett gelöscht**.
`TypstWorld` ist die einzige `World`-Implementation im Codebase — kein Grund,
Font-Init, File-Loading und `World`-Trait zweimal zu pflegen. Für
CLI-Einmal-Aufrufe bringt das Caching nichts, schadet aber auch nicht:
`TypstWorld::new` + ein einmaliger `compile_document` verhalten sich
funktional identisch zur alten `SimpleWorld::new` + `typst::compile`.

Damit die bestehenden Helper im File nicht alle umgebaut werden müssen,
bekommt `TypstWorld` einen zweiten Konstruktor für den CLI-typischen "nur
Template-Pfad"-Fall:

```rust
impl TypstWorld {
    /// Projekt-root + Projektname (GUI-Pfad, Worker-persistent).
    pub fn new(project_root: &Path, project_name: &str) -> Result<Self>;

    /// Template-Pfad zerlegen in (parent, file_stem). Convenience für
    /// `compile`, das historisch nur einen Pfad bekommt.
    pub fn from_template_path(template_path: &Path) -> Result<Self>;
}
```

`compile_to_intermediate_document` schrumpft damit auf drei Zeilen:

```rust
fn compile_to_intermediate_document(template_path: &Path)
    -> Result<PagedDocument>
{
    let mut world = TypstWorld::from_template_path(template_path)?;
    world.reload()?;                  // initiales Laden
    world.compile_document()
}
```

`compile_preview` / `compile_final` gehen denselben Weg, nur über
`TypstWorld::new(project_root, project_name)`.

Mehrkosten gegenüber dem alten `SimpleWorld`-Pfad: eine leere `HashMap` +
ein `Cell<u32>`-Initialisierung pro CLI-Aufruf. Unmessbar.

**Keine globale Singleton-World** — das würde Tests zerstören, die parallel
mit unterschiedlichen Projekt-Roots laufen. Jeder Aufrufer konstruiert
seine eigene `TypstWorld`.

---

## 0.4.4 — GUI-Worker nutzt eine persistente `TypstWorld`

`gui/background.rs` hält die World im Worker-Thread. Kein `Arc<Mutex>`, kein
`thread_local!` — ein Worker, eine World.

```rust
pub fn spawn(project_root: PathBuf, project_name: String) -> (…, …) {
    // …
    std::thread::spawn(move || {
        let pool = build_pool();
        let mut world = match TypstWorld::new(&project_root, &project_name) {
            Ok(w) => w,
            Err(e) => { /* send Error, exit */ return; }
        };
        while let Ok(task) = task_rx.recv() {
            match task {
                BackgroundTask::RenderPages { pages, pixel_per_pt } => {
                    // EIN Compile für diesen Task
                    if let Err(e) = world.reload() { /* send Error; continue */ }
                    let doc = match world.compile_document() { /* …  */ };
                    // N parallele Rasterisierungen, doc wird via Arc geteilt
                    let doc = Arc::new(doc);
                    for page in pages {
                        let doc = doc.clone();
                        let tx = result_tx.clone();
                        pool.spawn(move || {
                            match rasterize_page(&doc, page, pixel_per_pt) { /* … */ }
                        });
                    }
                }
            }
        }
    });
    // …
}
```

- `PagedDocument` ist `Send + Sync`, kann hinter `Arc` geteilt werden.
- Rayon parallelisiert **nur** die Rasterisierung. Der teure Compile läuft
  einmal pro Task im Worker-Thread.
- Falls mehrere `RenderPages`-Tasks aneinanderhängen (z.B. zweimal schnell
  hintereinander), profitiert der zweite Compile voll vom comemo-Cache.

---

## 0.4.5 — Metriken

Die existierende `Timings`-Struktur in `gui/state/timings.rs` bekommt zwei
zusätzliche Felder:

- `typst_compile: Duration` — Zeit für `compile_document` pro Task.
- `typst_rasterize_avg: Duration` — Durchschnitt pro Seite.

`overlay::show_timings_overlay` zeigt sie zusätzlich an. So kann man im F2-Overlay
live sehen, wie der erste Compile ~1 s ist und die folgenden auf ~100 ms fallen.

---

## Tests

### Unit-Tests in `src/output/typst_world.rs`

- `new_initializes_empty_slots`.
- `compile_document_succeeds_on_minimal_template`.
- `reload_keeps_unchanged_slots` — erstmal compilen, dann `reload()`,
  dann prüfen, dass `slots[main_id].source` **dieselbe** `Source` referenziert
  (via `Arc::ptr_eq` auf dem internen Arc, oder über einen Zähler "bytes_read").
- `reload_invalidates_changed_main` — Template-Datei neu schreiben, `reload()`,
  `compile_document()` sieht neuen Inhalt.
- `reload_invalidates_changed_image` — Bild-Datei austauschen, `reload()`,
  der `file()`-Call liefert neue `Bytes`.
- `second_compile_is_faster_than_first` — rein zeitbasiert, Toleranz grob
  (Faktor 3 schneller), damit CI nicht flaky wird. Optional `#[ignore]` mit
  `cargo test --ignored`.

### Integrationstest in `tests/`

Ein End-to-End-Test, der ein kleines Fixture-Projekt (1 Template + 2 Bilder)
zweimal compiliert und die zweite Compile-Zeit misst. Soll zeigen, dass
**beim zweiten Compile keine Datei neu vom Disk gelesen wird** — das können
wir über einen Test-Hook messen (Counter in `TypstWorld` für `fs::read`-Calls
hinter einem `#[cfg(test)]`).

---

## Akzeptanzkriterien

- [ ] `cargo build` und `cargo build --features gui` ohne Warnings.
- [ ] `cargo clippy -- -D warnings` sauber, `cargo fmt --check` sauber.
- [ ] `cargo test` inkl. neuer Tests grün.
- [ ] `SimpleWorld` ist aus dem Codebase **verschwunden**; `TypstWorld` ist
      die einzige `World`-Implementation. `grep -r SimpleWorld src/` leer.
- [ ] In einem echten Projekt mit ≥ 5 Seiten und ≥ 10 Bildern: zweiter
      `compile_document`-Aufruf ist **mindestens 5× schneller** als der erste
      (nachprüfbar via F2-Timings-Overlay).
- [ ] `render_pages` wird nicht mehr pro Seite im Parallelismus aufgerufen;
      `background.rs` macht einen Compile + N parallele Rasterisierungen.
- [ ] CLI-Pfade (`compile`, `compile_preview`, `compile_final`) funktionieren
      unverändert aus Nutzersicht; bestehende CLI-Tests grün. Intern gehen sie
      alle über `TypstWorld`.

## Was NICHT in dieser Phase gehört

- **Keine** Parallel-Compiles über mehrere Rayon-Worker. Ein Compile pro Task
  reicht und kollidiert nicht mit dem Caching.
- **Kein** globaler `TypstWorld`-Singleton. Tests müssen isoliert bleiben.
- **Kein** Blake3-Hash als Fallback neben mtime/len — nur wenn später tatsächlich
  Bit-Rot beobachtet wird.
- **Keine** präventive Invalidation beim Fenster-Fokus o.ä. — mtime reicht.
- **Keine** Änderung am YAML- oder Command-Layer; rein `src/output/`.
- **Keine** Rasterisierungs-Cache. Rasterisierung ist bereits billig; nur
  Compile ist teuer, und Compile cached Typst selbst.
