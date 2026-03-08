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
├── .gitignore                     # .fotobuch/ + *.pdf + final.typ
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
├── urlaub.typ                     # Template für urlaub (getrackt)
├── hochzeit.yaml                  # Branch fotobuch/hochzeit
├── hochzeit.typ                   # Template für hochzeit (getrackt)
└── final.typ                      # Generierte Kopie bei --release (nicht getrackt)
```

Cache-Ordner und `final.typ` werden **nicht** von Git getrackt (`.gitignore`). Pro Projekt werden `{name}.yaml`, `{name}.typ` und `.gitignore` getrackt. Jeder Branch trackt nur **seine eigenen** Dateien.

## Lifecycle

### Erstes Projekt: `fotobuch project new urlaub --width 420 --height 297 --bleed 3`

Erkennung: Kein `fotobuch/*`-Branch vorhanden, kein passendes YAML → frisches Repo.

1. Ordner `mein-fotobuch/` anlegen (oder aktuellen verwenden falls `--here`)
2. `git init`
3. `.gitignore` schreiben (`.fotobuch/`, `*.pdf`, `final.typ`)
4. `urlaub.yaml` mit Default-Config erstellen
5. `urlaub.typ` — Typst-Template mit `#let is_final = false` erstellen
6. `.fotobuch/cache/urlaub/preview/` und `.../final/` anlegen
7. `git checkout -b fotobuch/urlaub`
8. `git add urlaub.yaml urlaub.typ .gitignore`
9. `git commit -m "new: project urlaub (420x297mm, 3mm bleed)"`

### Weiteres Projekt: `fotobuch project new hochzeit`

Erkennung: Aktueller Branch ist `fotobuch/*` → existierendes Repo.

1. Validieren: `hochzeit.yaml` existiert noch nicht, Branch `fotobuch/hochzeit` existiert noch nicht
2. `hochzeit.yaml` mit Default-Config erstellen (Dimensionen aus dem aktiven Projekt übernehmen oder explizit angeben)
3. `hochzeit.typ` — Typst-Template erstellen (neues netrales standard template erstellen wie beim allerersten new aufruf, mit switch für "is_final")
4. `.fotobuch/cache/hochzeit/preview/` und `.../final/` anlegen
5. `git checkout -b fotobuch/hochzeit` (ausgehend vom aktuellen Branch)
6. Dateien des vorherigen Projekts aus dem Index entfernen: `git rm --cached urlaub.yaml urlaub.typ`
7. `git add hochzeit.yaml hochzeit.typ`
8. `git commit -m "new: project hochzeit (420x297mm, 3mm bleed)"`

**Wichtig**: Jeder Branch trackt nur **seine eigenen** Dateien (`{name}.yaml`, `{name}.typ`). `.gitignore` ist auf allen Branches gleich.

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

### Status Aufruf nennt aktiven projectnamen

```
fotobuch status
on project mein_supadupa_fotobuch
...
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

- `new()` wird zu `project_new()`  im Modul `commands/project/new.rs`
- Neue Logik: Repo-Erkennung (frisch vs. bestehend), Branch-Erstellung, YAML-Name aus Projektname
- `NewConfig` bekommt kein `name`-Feld mehr für den Ordner — der Projektname bestimmt Branch + YAML-Name

### Neues Modul: `src/commands/project/new.rs`

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

Pro Projekt gibt es **ein** Template `{name}.typ` das vom User gepflegt wird. Es enthält die gesamte Render-Logik inklusive Conditionals für Preview vs. Final:

```typst
#let data = yaml("urlaub.yaml")
#let is_final = false

// Layout-Logik ...
#if not is_final {
  // Wasserzeichen, Seitenzahlen, niedrigere Auflösung
}
```

Bei `build --release` erzeugt der Code eine **nicht-getrackte Kopie** `final.typ`:

1. `{name}.typ` lesen
2. `#let is_final = false` → `#let is_final = true` ersetzen
3. YAML-Pfad ggf. anpassen
4. Als `final.typ` schreiben (wird bei jedem Release überschrieben)

`final.typ` steht in `.gitignore`, ist aber auf Disk einsehbar zum Debuggen.

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
| 3 | `commands/project/new.rs` — `new` | Erstes + weiteres Projekt, Branch-Erstellung |
| 4 | `commands/project/list.rs` und `commands/project/switch` — `list` und `switch` | Branch-Wechsel und Auflistung |
| 5 | `state_manager.rs` — Projekt-Erkennung | Branch-Name → YAML-Name → Cache-Pfad |
| 6 | Cache-Pfade anpassen | `{projektname}/` Unterordner |
| 7 | Typst-Template + final.typ-Generierung | `{name}.typ` bei new, `final.typ` bei --release |
| 8 | CLI-Integration | clap Subcommand-Gruppe `project` |
