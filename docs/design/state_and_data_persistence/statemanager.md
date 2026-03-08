# StateManager

Stand: 2026-03-08

## Zweck

Der StateManager ist die zentrale Schnittstelle zwischen Commands und dem persistierten Projektzustand. Er kapselt:

- **YAML laden** mit automatischer Projekt-Erkennung aus dem Git-Branch
- **User-Änderungen erkennen und committen** beim Öffnen
- **State-Zugriff** für Commands (lesen + schreiben)
- **Diff-Erkennung** zwischen Ausgangszustand und aktuellem State
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
├─ finish(msg)  ← für schreibende Commands
│   ├─ Diff(last_state, state) → wenn leer: return (nichts zu committen)
│   ├─ YAML schreiben
│   ├─ git add + commit("{msg} — {summary}")
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

    pub state: ProjectState,              // pub für disjoint borrows
    baseline: ProjectState,               // Baseline seit open() (nach auto-commit)
    last_build_state: Option<ProjectState>, // State beim letzten build:/rebuild:-Commit
    raw_config: Value,
    committed: bool,
}
```

`state` ist bewusst `pub` — so kann der Rust-Compiler disjoint borrows auf `mgr.state.photos` und `mgr.state.layout` gleichzeitig erlauben. Methoden wie `state()` / `state_mut()` würden den gesamten `StateManager` borrowen und das verhindern.

### Zwei Vergleichs-Baselines

| Feld | Befüllt in | Zweck |
|------|-----------|-------|
| `baseline` | `open()`, nach auto-commit | Erkennt **programmatische** Änderungen seit dem Öffnen (für `finish()`, `Drop`-Warnung) |
| `last_build_state` | `open()`, aus letztem `build:`/`rebuild:`-Commit | Erkennt **Nutzer-Änderungen seit dem letzten Build** (für `incremental_build`, `release_build`) |

**Warum `last_build_state` statt `baseline`?**

Das Problem mit `baseline` als Vergleichspunkt für `incremental_build`:

```
1. build          → HEAD = "build: 3 pages"   state = S_build
2. User editiert  → Disk-YAML = S_edit (area_weight geändert)
3. fotobuch build → open() auto-committet S_edit
                    baseline = S_edit          ← FALSCH als Build-Basis
                    incremental_build: S_edit vs S_edit → "Nothing to do"
                    Aber area_weight hat sich geändert! Bug.
```

Mit `last_build_state = S_build` als Vergleichspunkt wird der Unterschied korrekt erkannt.

**Immunität gegen "Edit + Revert":**

```
1. build          → HEAD = "build: 3 pages"   last_build_state = S_build
2. User editiert  → auto-commit "chore: manual edits"
3. User revertiert → auto-commit "chore: manual edits"
4. fotobuch build → last_build_state = S_build, state = S_build
                    has_changes_since_last_build() → false → "Nothing to do" ✓
```

`baseline` wäre hier auch S_build (weil das Revert auto-committet wurde), aber man sollte sich nicht darauf verlassen — `last_build_state` macht die Intention explizit.

## Lifecycle (aktualisiert)

```
open(project_root)
│
├─ 1. Git-Branch lesen → Projektname ableiten
├─ 2. {projektname}.yaml laden → self.state
├─ 3. Letzte committed Version laden (git2: HEAD:{projektname}.yaml)
├─ 4. Diff(committed, loaded) → wenn nicht leer:
│     YAML committen mit "chore: manual edits — {summary}"
├─ 5. self.baseline = self.state.clone()
├─ 6. Letzten build:/rebuild:-Commit suchen (git-log rückwärts)
│     → self.last_build_state = Some(state aus diesem Commit) oder None
│
▼ Command arbeitet mit mgr.state (pub field)
│
├─ finish(msg)  ← für schreibende Commands
│   ├─ Diff(baseline, state) → wenn leer: return (nichts zu committen)
│   ├─ YAML schreiben
│   ├─ git add + commit("{msg} — {summary}")
│   └─ self.committed = true
│
▼ Drop
    └─ Warnung wenn uncommitted programmatische Änderungen vorliegen
```

**Suche nach letztem Build-Commit (Schritt 6):**

Git-Log rückwärts traversieren (`repo.revwalk()`), für jeden Commit Message prüfen ob sie mit `"build:"` oder `"rebuild:"` beginnt. Beim ersten Treffer YAML-Blob laden (`HEAD~N:{name}.yaml`). Kein Treffer → `last_build_state = None` (dann fallen `has_changes_since_last_build()` und `modified_pages()` auf `baseline` zurück).

**Schwachstelle:** Abhängigkeit von Commit-Message-Format. Dieses Format ist aber projektintern und wird nur von fotobuch selbst erzeugt — daher akzeptabel.

## API

```rust
impl StateManager {
    /// Öffnet das Projekt: YAML laden, User-Diff committen, Build-Baseline laden.
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

    /// Speichert YAML + committet, falls sich state seit open() geändert hat.
    /// Konsumiert den Manager.
    pub fn finish(mut self, message: &str) -> Result<()>

    /// true wenn sich state programmatisch seit open() geändert hat.
    /// Basis: `baseline` (nach auto-commit). Genutzt von finish() und Drop.
    pub fn has_changes_since_open(&self) -> bool

    /// true wenn state vom letzten build:/rebuild:-Commit abweicht.
    /// Basis: `last_build_state` (oder `baseline` wenn kein Build-Commit vorhanden).
    /// Genutzt von: release_build (clean-check), incremental_build.
    pub fn has_changes_since_last_build(&self) -> bool

    /// Welche Seiten (1-basiert) müssen neu gebaut werden?
    /// Vergleicht gegen last_build_state (nicht baseline).
    pub fn modified_pages(&self) -> Vec<usize>
}
```

### Umbenennung `has_changes` → `has_changes_since_open`

Der alte Name `has_changes()` war mehrdeutig. `has_changes_since_open()` macht den Vergleichspunkt explizit. `has_changes_since_last_build()` ist das neue Pendant für Build-Commands.

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

Kein I/O im Drop — nur eine Warnung. Der eigentliche Commit muss über `finish()` erfolgen.

## Auswirkungen auf bestehende Pläne

| Modul | Änderung |
|-------|----------|
| `git.rs` | Wird intern vom StateManager genutzt, nicht direkt von Commands |
| `project/diff.rs` (aus build/status Plan) | Wird zu `StateDiff` im StateManager |
| `commands/build.rs` | Nutzt `StateManager::open()` + `finish()` |
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
| 4 | finish / Drop |
| 5 | Hilfsmethoden — cache_dir, raw_config, has_changes |
| 6 | Bestehende Commands auf StateManager umstellen |
