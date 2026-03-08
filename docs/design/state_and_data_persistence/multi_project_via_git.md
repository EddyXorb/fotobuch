# Multi-Projekt-Verwaltung via Git-Branches

Stand: 2026-03-08

## Idee

Jedes Fotobuch-Projekt lebt auf einem eigenen Git-Branch `fotobuch/<projektname>`. Pro Projekt gibt es eine eigene YAML-Datei `<projektname>.yaml`. Der aktive Branch bestimmt, welches Projekt bearbeitet wird — kein `--project`-Flag nötig.

## Projektnamen

Erlaubt: `[a-zA-Z][a-zA-Z0-9._-]*`, maximal 50 Zeichen.

Verboten: `..` (Git-Restriction), rein numerisch, reservierte Namen.

Validierung in einer zentralen Funktion die von `project new` und `StateManager::open` genutzt wird.

## Verzeichnisstruktur

```
mein-fotobuch/
├── .git/
├── .gitignore                     # .fotobuch/ + *.pdf
├── .fotobuch/
│   └── cache/
│       ├── urlaub/                # Cache pro Projekt
│       │   ├── preview/
│       │   │   └── Strand/{id}.jpg
│       │   └── final/
│       │       └── Strand/{id}.jpg
│       └── hochzeit/
│           ├── preview/
│           └── final/
├── urlaub.yaml                    # Branch fotobuch/urlaub
├── hochzeit.yaml                  # Branch fotobuch/hochzeit
├── fotobuch_preview.typ
└── fotobuch_final.typ
```

Cache-Ordner werden **nicht** von Git getrackt (`.gitignore`). YAML, Typst-Templates und `.gitignore` werden getrackt.

## Lifecycle

### Erstes Projekt: `fotobuch project new urlaub --width 420 --height 297 --bleed 3`

Erkennung: Kein `fotobuch/*`-Branch vorhanden, kein passendes YAML → frisches Repo.

1. Ordner `mein-fotobuch/` anlegen (oder aktuellen verwenden falls `--here`)
2. `git init`
3. `.gitignore` schreiben, Typst-Templates schreiben
4. `urlaub.yaml` mit Default-Config erstellen
5. `.fotobuch/cache/urlaub/preview/` und `.../final/` anlegen
6. `git checkout -b fotobuch/urlaub`
7. `git add urlaub.yaml .gitignore fotobuch_preview.typ fotobuch_final.typ`
8. `git commit -m "new: project urlaub (420x297mm, 3mm bleed)"`

### Weiteres Projekt: `fotobuch project new hochzeit`

Erkennung: Aktueller Branch ist `fotobuch/*` → existierendes Repo.

1. Validieren: `hochzeit.yaml` existiert noch nicht, Branch `fotobuch/hochzeit` existiert noch nicht
2. `hochzeit.yaml` mit Default-Config erstellen (Dimensionen aus dem aktiven Projekt übernehmen oder explizit angeben)
3. `.fotobuch/cache/hochzeit/preview/` und `.../final/` anlegen
4. `git checkout -b fotobuch/hochzeit` (ausgehend vom aktuellen Branch)
5. Das YAML des vorherigen Projekts aus dem Index entfernen: `git rm --cached urlaub.yaml` (Datei bleibt auf Disk, verschwindet aus diesem Branch)
6. `git add hochzeit.yaml`
7. `git commit -m "new: project hochzeit (420x297mm, 3mm bleed)"`

**Wichtig**: Jeder Branch trackt nur **sein eigenes** YAML. Typst-Templates und `.gitignore` sind auf allen Branches gleich.

### Projekt wechseln: `fotobuch project switch urlaub`

1. Prüfen ob uncommitted Changes am aktuellen YAML → ggf. auto-commit (StateManager-Logik)
2. `git switch fotobuch/urlaub`
3. Fertig — StateManager erkennt beim nächsten Command automatisch `urlaub.yaml`

### Projekte auflisten: `fotobuch project list`

1. Alle Branches mit Prefix `fotobuch/` via git2 auflisten
2. Aktuellen Branch markieren
3. Ausgabe:

```
  hochzeit
* urlaub (active)
```

## Projekt-Erkennung im StateManager

```
StateManager::open(project_root)
  ├─ git2: aktuellen Branch-Namen lesen
  ├─ Prefix "fotobuch/" prüfen → Fehler wenn nicht
  ├─ Projektname = Branch-Name ohne Prefix
  ├─ YAML-Pfad = project_root / "{projektname}.yaml"
  ├─ Cache-Pfad = project_root / ".fotobuch/cache/{projektname}/"
  └─ weiter mit StateManager-Logik (laden, User-Diff, etc.)
```

## Änderungen an bestehenden Modulen

### `src/commands/new.rs`

- `new()` wird zu `project_new()` oder bleibt `new()` im Modul `commands/project.rs`
- Neue Logik: Repo-Erkennung (frisch vs. bestehend), Branch-Erstellung, YAML-Name aus Projektname
- `NewConfig` bekommt kein `name`-Feld mehr für den Ordner — der Projektname bestimmt Branch + YAML-Name

### Neues Modul: `src/commands/project.rs`

```rust
pub fn new(project_root: &Path, config: &NewConfig) -> Result<NewResult>
pub fn switch(project_root: &Path, name: &str) -> Result<()>
pub fn list(project_root: &Path) -> Result<Vec<ProjectInfo>>

pub fn validate_project_name(name: &str) -> Result<()>

pub struct ProjectInfo {
    pub name: String,
    pub active: bool,
}
```

### `src/git.rs`

Ersetzt `Command::new("git")` durch `git2`-Crate. Neue Funktionen:

```rust
pub fn current_branch(repo: &git2::Repository) -> Result<String>
pub fn create_branch(repo: &git2::Repository, name: &str) -> Result<()>
pub fn switch_branch(repo: &git2::Repository, name: &str) -> Result<()>
pub fn list_branches_with_prefix(repo: &git2::Repository, prefix: &str) -> Result<Vec<String>>
```

### `src/state_manager.rs`

Leitet YAML-Dateiname und Cache-Pfad aus dem Branch-Namen ab. Siehe [statemanager.md](statemanager.md).

### CLI-Ebene (clap)

```
fotobuch project new <name> [--width] [--height] [--bleed]
fotobuch project switch <name>
fotobuch project list
```

`project` wird ein clap-Subcommand mit eigenen Sub-Subcommands.

### Typst-Templates

Die Templates lesen das YAML dynamisch. Der YAML-Dateiname muss entweder:
- als Variable übergeben werden (Typst `--input yaml=urlaub.yaml`)
- oder als Symlink `fotobuch.yaml → urlaub.yaml` bereitgestellt werden

**Empfehlung**: Symlink beim `StateManager::open()` aktualisieren — einfach, transparent, Templates bleiben unverändert.

### Cache-Pfade

`cache/common.rs` → `cache_dir()` muss den Projektnamen einbeziehen:

```
.fotobuch/cache/{projektname}/preview/{group}/{local_id}.jpg
.fotobuch/cache/{projektname}/final/{group}/{local_id}.jpg
```

## Implementierungsreihenfolge

| # | Schritt | Details |
|---|---------|---------|
| 1 | `validate_project_name()` | Regex-Validierung, zentral nutzbar |
| 2 | `git.rs` auf git2 umstellen | Branch-Operationen, kein `Command::new("git")` mehr |
| 3 | `commands/project.rs` — `new` | Erstes + weiteres Projekt, Branch-Erstellung |
| 4 | `commands/project.rs` — `switch` + `list` | Branch-Wechsel und Auflistung |
| 5 | `state_manager.rs` — Projekt-Erkennung | Branch-Name → YAML-Name → Cache-Pfad |
| 6 | Cache-Pfade anpassen | `{projektname}/` Unterordner |
| 7 | Typst-Symlink | `fotobuch.yaml → {projekt}.yaml` bei open() |
| 8 | CLI-Integration | clap Subcommand-Gruppe `project` |
