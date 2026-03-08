# StateManager

Stand: 2026-03-08

## Zweck

Der StateManager ist die zentrale Schnittstelle zwischen Commands und dem persistierten Projektzustand. Er kapselt:

- **YAML laden** mit automatischer Projekt-Erkennung aus dem Git-Branch
- **User-Änderungen erkennen und committen** beim Öffnen
- **State-Zugriff** für Commands (lesen + schreiben)
- **Diff-Erkennung** zwischen letztem Checkpoint und aktuellem State
- **YAML speichern + Git-Commit** mit aussagekräftiger Summary

Ohne den StateManager müsste jedes Command diese Logik selbst implementieren.

## Ort

`src/state_manager.rs` — eigenständiges Modul auf Top-Level, da es von allen Commands genutzt wird.

## Lifecycle

```
open(project_root)
│
├─ 1. Git-Branch lesen → Projektname ableiten
├─ 2. {projektname}.yaml laden → self.state
├─ 3. Letzte committed Version laden (git2: HEAD:{projektname}.yaml)
├─ 4. Diff(committed, loaded) → wenn nicht leer:
│     YAML committen mit "chore: manual edits — {summary}"
├─ 5. self.last_state = self.state.clone()
│
▼ Command arbeitet mit mgr.state (pub field)
│
├─ checkpoint(msg)  ← optional, für Zwischen-Commits
│   ├─ Diff(last_state, state) → wenn leer: return
│   ├─ YAML schreiben
│   ├─ git add + commit("{msg} — {summary}")
│   └─ last_state = state.clone()
│
├─ finish(msg)  ← für schreibende Commands
│   ├─ checkpoint(msg)
│   └─ self.committed = true
│
▼ Drop
    └─ Warnung wenn uncommitted programmatische Änderungen vorliegen
```

## Struct

```rust
pub struct StateManager {
    project_root: PathBuf,
    project_name: String,
    repo: git2::Repository,

    pub state: ProjectState,       // pub für disjoint borrows
    last_state: ProjectState,      // privat, für Diff
    committed: bool,
}
```

`state` ist bewusst `pub` — so kann der Rust-Compiler disjoint borrows auf `mgr.state.photos` und `mgr.state.layout` gleichzeitig erlauben. Methoden wie `state()` / `state_mut()` würden den gesamten `StateManager` borrowen und das verhindern.

## API

```rust
impl StateManager {
    /// Öffnet das Projekt: YAML laden, User-Diff committen.
    pub fn open(project_root: &Path) -> Result<Self>

    /// Projektname (abgeleitet aus Branch).
    pub fn project_name(&self) -> &str

    /// Pfad zum Cache-Verzeichnis des aktiven Projekts.
    pub fn cache_dir(&self) -> PathBuf
    pub fn preview_cache_dir(&self) -> PathBuf
    pub fn final_cache_dir(&self) -> PathBuf

    /// YAML-Pfad des aktiven Projekts.
    pub fn yaml_path(&self) -> PathBuf

    /// Raw serde_yaml::Value der config-Sektion (für config-Command Default-Erkennung).
    pub fn raw_config(&self) -> &serde_yaml::Value

    /// Zwischenstand speichern + committen (nur wenn Änderungen vorliegen).
    pub fn checkpoint(&mut self, message: &str) -> Result<()>

    /// Finaler Commit. Konsumiert den Manager.
    pub fn finish(mut self, message: &str) -> Result<()>

    /// Gibt true zurück wenn sich state seit last_state geändert hat.
    pub fn has_changes(&self) -> bool
}
```

## StateDiff

Internes Hilfs-Struct für die Zusammenfassung von Änderungen.

```rust
struct StateDiff {
    config_changes: usize,
    photos_added: usize,
    photos_removed: usize,
    pages_added: usize,
    pages_removed: usize,
    pages_modified: usize,
}
```

`StateDiff::compute(old, new)` vergleicht zwei `ProjectState`-Instanzen. `summary()` erzeugt einen einzeiligen Text wie:

> changed 2 configs, added 15 photos, added 4 pages

`is_empty()` gibt true zurück wenn alle Felder 0 sind.

### Vergleichslogik

- **Config**: Beide zu `serde_yaml::Value` serialisieren, rekursiv Leaf-Werte vergleichen, Unterschiede zählen
- **Photos**: Group-Namen und File-IDs als Sets vergleichen (add/remove Count)
- **Pages**: Seitenanzahl vergleichen (add/remove), dann pro überlappender Seite Slot-IDs vergleichen (modified Count)

## Benutzung in Commands

### Schreibendes Command (build, place, remove, rebuild)

```rust
pub fn place(project_root: &Path, config: &PlaceConfig) -> Result<PlaceResult> {
    let mut mgr = StateManager::open(project_root)?;

    let unplaced = find_unplaced(&mgr.state);
    // ... Logik, mutiert mgr.state.layout ...

    mgr.finish("feat: place photos")?;
    Ok(result)
}
```

### Command mit Zwischen-Commits (build)

```rust
pub fn build(project_root: &Path, config: &BuildConfig) -> Result<BuildResult> {
    let mut mgr = StateManager::open(project_root)?;

    // Solver laufen lassen, Layout schreiben
    run_solver(&mut mgr.state)?;
    mgr.checkpoint("feat: compute layout")?;

    // Cache generieren, Previews rendern
    ensure_previews(&mgr)?;
    mgr.finish("feat: build previews")?;
    Ok(result)
}
```

### Lesendes Command (config, status)

```rust
pub fn config(project_root: &Path) -> Result<ConfigResult> {
    let mgr = StateManager::open(project_root)?;
    // open() hat eventuelle User-Edits bereits committet

    Ok(ConfigResult {
        resolved: mgr.state.config.clone(),
        raw: mgr.raw_config().clone(),
    })
    // Drop: keine programmatischen Änderungen → keine Warnung
}
```

## Drop-Verhalten

```rust
impl Drop for StateManager {
    fn drop(&mut self) {
        if !self.committed {
            let diff = StateDiff::compute(&self.last_state, &self.state);
            if !diff.is_empty() {
                eprintln!(
                    "warning: StateManager dropped with uncommitted changes: {}",
                    diff.summary()
                );
            }
        }
    }
}
```

Kein I/O im Drop — nur eine Warnung. Der eigentliche Commit muss über `checkpoint()` oder `finish()` erfolgen.

## Auswirkungen auf bestehende Pläne

| Modul | Änderung |
|-------|----------|
| `git.rs` | Wird intern vom StateManager genutzt, nicht direkt von Commands |
| `project/diff.rs` (aus build/status Plan) | Wird zu `StateDiff` im StateManager |
| `commands/build.rs` | Nutzt `StateManager::open()` + `checkpoint()` + `finish()` |
| `commands/rebuild.rs` | Nutzt `StateManager::open()` + `finish()` |
| `commands/place.rs` | Nutzt `StateManager::open()` + `finish()` |
| `commands/remove.rs` | Nutzt `StateManager::open()` + `finish()` |
| `commands/config.rs` | Nutzt `StateManager::open()` (kein finish) |
| `commands/status.rs` | Nutzt `StateManager::open()` + `has_changes()` |
| Cache-Pfade | Über `mgr.preview_cache_dir()` / `mgr.final_cache_dir()` |

## Implementierungsreihenfolge

| # | Schritt |
|---|---------|
| 1 | `StateDiff` — compute + summary + is_empty |
| 2 | `StateManager` Grundstruktur — open, yaml laden, Branch-Erkennung |
| 3 | User-Diff beim open — committed vs. loaded vergleichen + auto-commit |
| 4 | checkpoint / finish / Drop |
| 5 | Hilfsmethoden — cache_dir, raw_config, has_changes |
| 6 | Bestehende Commands auf StateManager umstellen |
