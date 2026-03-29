# StateManager

## Zweck

Zentrale Schnittstelle zwischen Commands und dem persistierten Projektzustand. Kapselt:

- YAML laden mit automatischer Projekt-Erkennung aus dem Git-Branch
- User-Änderungen beim Öffnen erkennen und committen
- State-Zugriff für Commands (lesen + schreiben)
- YAML speichern + Git-Commit mit aussagekräftiger Summary

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

`state` ist bewusst `pub` — so erlaubt der Rust-Compiler disjoint borrows auf `mgr.state.photos` und `mgr.state.layout` gleichzeitig.

## Lifecycle

```
open(project_root)
├─ Git-Branch lesen → Projektname ableiten
├─ {projektname}.yaml laden → self.state
├─ Letzte committed Version laden (HEAD:{projektname}.yaml)
├─ Diff(committed, loaded) → wenn nicht leer: auto-commit "chore: manual edits — {summary}"
├─ self.baseline = self.state.clone()
└─ Letzten build:/rebuild:-Commit suchen → self.last_build_state

Command arbeitet mit mgr.state

finish(msg)  ← für schreibende Commands
├─ Diff(baseline, state) → wenn leer: return
├─ YAML schreiben
└─ git add + commit("{msg} — {summary}")

Drop
└─ Warnung wenn uncommitted programmatische Änderungen vorliegen
```

## Zwei Vergleichs-Baselines

| Feld | Befüllt | Zweck |
|---|---|---|
| `baseline` | `open()`, nach auto-commit | Erkennt programmatische Änderungen für `finish()` |
| `last_build_state` | Letzter `build:`/`rebuild:`-Commit | Erkennt Nutzer-Änderungen seit letztem Build |

**Warum `last_build_state`?** `baseline` wäre nach einem auto-commit identisch mit dem aktuellen State → inkrementeller Build würde fälschlich "Nothing to do" melden, obwohl sich `area_weight` geändert hat.

## API

```rust
pub fn open(project_root: &Path) -> Result<Self>
pub fn project_name(&self) -> &str
pub fn cache_dir(&self) -> PathBuf
pub fn preview_cache_dir(&self) -> PathBuf
pub fn final_cache_dir(&self) -> PathBuf
pub fn yaml_path(&self) -> PathBuf
pub fn raw_config(&self) -> &serde_yaml::Value
pub fn finish(mut self, message: &str) -> Result<()>
pub fn has_changes_since_open(&self) -> bool
pub fn has_changes_since_last_build(&self) -> bool
pub fn modified_pages(&self) -> Vec<usize>
```

## StateDiff

Internes Hilfs-Struct für Commit-Summaries.

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

`summary()` erzeugt: `"changed 2 configs, added 15 photos, added 4 pages"`.

Vergleichslogik: Config → `serde_yaml::Value`-Vergleich; Photos → Set-Differenz nach IDs; Pages → Slot-IDs pro überlappender Seite.

## Drop-Verhalten

Kein I/O im Drop — nur Warnung auf stderr wenn uncommitted programmatische Änderungen vorliegen.
