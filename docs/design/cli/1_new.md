# Implementation Plan: `fotobuch project new`

Stand: 2026-03-08

## Überblick

`fotobuch project new <name>` erstellt ein neues Fotobuch-Projekt. Mehrere Projekte
koexistieren im selben Git-Repository auf separaten Branches (`fotobuch/<name>`).
Jedes Projekt hat eine eigene YAML-Datei (`<name>.yaml`) und ein eigenes Typst-Template
(`<name>.typ`). Das finale Template (`final.typ`) wird erst bei `build --release` generiert
und ist nicht getrackt.

## CLI-Interface

```text
$ fotobuch project new --help
Create a new photobook project

Usage: fotobuch project new [OPTIONS] --width <MM> --height <MM> <NAME>

Arguments:
  <NAME>  Project name (used as branch name fotobuch/<name>, file name <name>.yaml)

Options:
      --width <MM>   Page width in mm
      --height <MM>  Page height in mm
      --bleed <MM>   Bleed margin in mm [default: 3]
  -h, --help         Print help
```

### Verwandte Subkommandos

```text
fotobuch project switch <name>   # Wechsel zu einem anderen Projekt (git checkout fotobuch/<name>)
fotobuch project list            # Zeigt alle vorhandenen Projekte (fotobuch/*-Branches)
```

## Abhängigkeiten

- `git2` crate (statt `std::process::Command`) für alle Git-Operationen
- `serde_yaml` (vorhanden)
- `dto_models::ProjectState` / `ProjectConfig` / `BookConfig` (vorhanden)

## Namensvalidierung

Vor jeder Operation wird der Projektname validiert:

```rust
pub fn validate_project_name(name: &str) -> Result<()>
```

Regeln:

- Muss mit `[a-zA-Z]` beginnen
- Darf nur `[a-zA-Z0-9._-]` enthalten
- Maximallänge: 50 Zeichen
- Darf nicht `..` enthalten (Pfadtraversal)
- Darf nicht `fotobuch` sein (reserviert als Branch-Präfix)
- Entspricht dem Regex `^[a-zA-Z][a-zA-Z0-9._-]{0,49}$` mit den obigen Ausnahmen

## Zwei Betriebsmodi

### Modus 1: Erstes Projekt (kein `fotobuch/*`-Branch existiert)

Erstellt einen neuen ordner mit dem namen des neuen projects.
Es ist nicht schlimm diesen ordner umzubennen - das soll als nachricht beim ersten aufruf von
new auch explizit genannt werden, dass er umbenannt werden kann, aber die yaml und typ dateien nicht.
So eine Art Begrüßung und einführung sollte stattfinden (nicht mehr als 10 zeilen erklärung), inklusive der erklärunge des generellen workflows und dass man die .yaml und .typ tweaken darf und bei bedarf änderungen rückgängig machen kann und alles getrackt wird.

1. **Verzeichnis anlegen** unter `<parent>/<name>/`
2. **Git initialisieren** via `git2::Repository::init()`
3. **`.gitignore` schreiben**:

   ```gitignore
   .fotobuch/
   *.pdf
   final.typ
   ```

4. **`<name>.yaml` schreiben** mit Seitenmassen aus `NewConfig` und Defaults
5. **`<name>.typ` schreiben** via `include_str!` aus `src/templates/fotobuch.typ`,
   `{name}`-Platzhalter durch den Projektnamen ersetzen
6. **Cache-Verzeichnisse anlegen**:
   - `.fotobuch/cache/<name>/preview/`
   - `.fotobuch/cache/<name>/final/`
7. **Branch erstellen**: `git checkout -b fotobuch/<name>` via `git2`
8. **Staging**: `git add .gitignore <name>.yaml <name>.typ`
9. **Commit**: `new: <name>, <W>x<H>mm, <B>mm bleed`
10. **Begrüßung** (siehe oben) ausgeben.

Resultierende Struktur:

```text
<name>/
├── .git/
├── .gitignore
├── <name>.yaml
├── <name>.typ
└── .fotobuch/
    └── cache/
        └── <name>/
            ├── preview/
            └── final/
```

### Modus 2: Weiteres Projekt (bereits auf einem `fotobuch/*`-Branch)

1. **Validieren**: Name darf nicht bereits als Branch `fotobuch/<name>` existieren
2. **`<name>.yaml` schreiben** im Repository-Root
3. **`<name>.typ` schreiben** mit `{name}`-Platzhalter ersetzt — frisches Standard-Template
   mit `#let is_final = false` für Preview/Final-Umschaltung
4. **Cache-Verzeichnisse anlegen**:
   - `.fotobuch/cache/<name>/preview/`
   - `.fotobuch/cache/<name>/final/`
5. **Branch erstellen**: `git checkout -b fotobuch/<name>` via `git2`
6. **Altes Projekt aus Index entfernen**: `git rm --cached <old>.yaml <old>.typ`
   (Dateien bleiben auf der Platte, nur der Index-Eintrag wird entfernt)
7. **Neues Projekt stagen**: `git add <name>.yaml <name>.typ`
8. **Commit**: `new: <name>, <W>x<H>mm, <B>mm bleed`

Jeder Branch `fotobuch/<name>` trackt ausschließlich die Dateien des zugehörigen Projekts.
Wechsel zwischen Projekten via `fotobuch project switch <name>` führt `git checkout
fotobuch/<name>` aus — der Working Tree zeigt dann genau `<name>.yaml` und `<name>.typ`.

## Typst-Template

Das Template wird aus `src/templates/fotobuch.typ` via `include_str!` eingebettet.
Zur Projektanlage werden die `{name}`-Platzhalter durch den tatsächlichen Projektnamen ersetzt.

Template-Vorlage (`src/templates/fotobuch.typ`):

```typst
#let is_final = false
#let data = yaml("{name}.yaml")

// Cache-Pfad je nach Modus
#let cache_prefix = if is_final {
  ".fotobuch/cache/{name}/final/"
} else {
  ".fotobuch/cache/{name}/preview/"
}

// Seitengröße aus YAML
#set page(
  width: data.config.book.page_width_mm * 1mm,
  height: data.config.book.page_height_mm * 1mm,
  margin: 0mm,
)

// Seiten rendern
#for page_data in data.layout [
  // Slots aus page_data.slots iterieren
  // Preview: Wasserzeichen via #place() + #rotate() + #text()
  // Final (is_final == true): kein Wasserzeichen
]
```

Der Switch `#let is_final = false` steuert das Verhalten:

- `false` (default, Preview): Wasserzeichen, Annotationen, Seitenzahlen gemäß Config
- `true` (wird in `final.typ` auf `true` gesetzt): kein Wasserzeichen, druckfertig

`final.typ` wird bei `build --release` generiert (nicht getrackt, in `.gitignore`).

## Signaturen

```rust
// src/commands/project_new.rs

pub struct NewConfig {
    pub name: String,
    pub width_mm: f64,
    pub height_mm: f64,
    pub bleed_mm: f64,
}

pub struct NewResult {
    pub project_root: PathBuf,
    pub branch: String,       // "fotobuch/<name>"
    pub yaml_path: PathBuf,
    pub typ_path: PathBuf,
}

pub struct ProjectInfo {
    pub name: String,
    pub branch: String,       // "fotobuch/<name>"
    pub is_current: bool,
}

/// Erstellt ein neues Projekt. `parent_dir_or_root` ist entweder das
/// Elternverzeichnis (Modus 1) oder der Repository-Root (Modus 2).
pub fn project_new(parent_dir_or_root: &Path, config: &NewConfig) -> Result<NewResult>

/// Validiert den Projektnamen gegen die Namensregeln.
pub fn validate_project_name(name: &str) -> Result<()>

/// Wechselt zum Branch `fotobuch/<name>` via git2.
pub fn project_switch(project_root: &Path, name: &str) -> Result<()>

/// Listet alle `fotobuch/*`-Branches mit is_current-Flag.
pub fn project_list(project_root: &Path) -> Result<Vec<ProjectInfo>>
```

## Neue Dateien

- `src/templates/fotobuch.typ` — Basis-Template mit `{name}`-Platzhaltern

## Modulstruktur

Alle `project`-Subkommandos leben in `src/commands/project.rs` (kein `mod.rs`, kein
Unterverzeichnis solange die Datei unter ~300 Zeilen bleibt). Wächst die Datei darüber
hinaus, wird sie in `src/commands/project/` aufgeteilt:

```text
src/commands/project/
    git.rs     # git2-Operationen (init, branch, add, commit, rm)
    yaml.rs    # YAML-Serialisierung, Defaults
    template.rs # Template-Substitution, include_str!
```

## Konventionen

- **Conventional Commits**: Jeder Teilschritt bekommt einen eigenen Commit
  (z.B. `feat: validate project name`, `feat: create project directory structure`,
  `test: add integration test for project new`).
- **Tests vor jedem Commit**: `cargo test` muss fehlerfrei durchlaufen.
- **Unit-Tests**: `#[cfg(test)]`-Block inline in `project.rs` für Namensvalidierung,
  Template-Substitution, Modus-Erkennung.
- **Integrationstests**: `tests/project_new.rs` prüft Verzeichnisstruktur, YAML-Inhalt,
  Branch-Name, `.gitignore`-Einträge, git2-Repository-Zustand.
- **`clippy --fix`** vor jedem Commit ausführen.
- **`cargo build`** regelmäßig, alle Warnings beheben.
- **Kein `mod.rs`**: Untermodule als `src/commands/project.rs` + `src/commands/project/`.
- **Kein `std::process::Command` für Git**: Ausschließlich `git2`-Crate verwenden.

## Tests

- Namensvalidierung: gültige Namen, zu kurz, falscher Start, Sonderzeichen, `..`, `fotobuch`
- Modus 1 (Erst-Projekt): Verzeichnisstruktur, YAML-Inhalt, `.gitignore`, Branch-Name,
  git-Commit vorhanden, Template-Inhalt mit korrektem `{name}`-Ersatz
- Modus 2 (Weiteres Projekt): Branch-Existenzcheck, `git rm --cached` für altes Projekt,
  neues Projekt im Index, Commit vorhanden
- `project_list`: gibt alle `fotobuch/*`-Branches zurück, `is_current` korrekt gesetzt
- `project_switch`: Branch-Wechsel via git2, Fehler bei nicht existierendem Projekt
- Fehler bei doppeltem Projektnamen (Branch existiert bereits)
