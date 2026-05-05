# Phase 7: Project Lifecycle (New / Switch / History) + Working-Dir-UX

**Ziel**: GUI-Parität mit der CLI-Familie `fotobuch project {new, list,
switch}` und `fotobuch history`. Zusätzlich eine UX-Antwort auf die
offene Frage „Woher kommt der Working Dir?", die für Anwender ohne
CLI/Git-Kenntnisse genauso funktioniert wie für Power-User.

**Voraussetzung**: Phasen 1–6 abgeschlossen. Toolbar, Toasts, Add-Dialog
laufen.

---

## Status quo

- Lib komplett vorhanden:
  - `commands::project::new::project_new(parent_dir, &NewConfig)`
  - `commands::project::list::project_list(repo_root)`
  - `commands::project::switch::project_switch(repo_root, name)`
  - `commands::history::history(repo_root, count)`
- Jedes Projekt = git-Branch `fotobuch/<name>` im selben Repo.
  „Working Dir" der GUI = **Repo-Root**, nicht das Projekt selbst.
- GUI heute (`gui/main.rs:12`): `std::env::current_dir()`. Funktioniert
  nur, wenn man aus dem Terminal im richtigen Ordner startet — für
  Doppelklick-Starter (Desktop-Icon) unbrauchbar.
- Es existiert noch keine GUI-State-Persistenz außerhalb des Projekts
  selbst. Alles, was über einen Lauf hinaus überleben soll, muss neu
  angelegt werden.

---

## 7.0 Kernentscheidung: Vault-Modell

Inspirationen aus etablierten Apps:

| App | Muster | Übernehmbar? |
|---|---|---|
| Obsidian | Beim ersten Start „Vault wählen", dann remembered. App-Settings unter OS-Config-Dir. | ✅ Direkt anwendbar — eine Vault = ein Repo. |
| VS Code | „Recent Folders" + Folder-Picker im Welcome. | ✅ Recent-List übernehmen. |
| GitHub Desktop | Liste hinzugefügter Repos, „Clone"/"Add existing". | ✅ Add-existing als Power-User-Pfad. |
| Lightroom | Default-Bibliothek `~/Pictures/Lightroom`, später moveable. | ✅ Sinnvoller Default. |
| Steam | Fixe `~/.local/share/Steam`-Library, Multi-Library als Power-Feature. | ⚠ Zu opinionated für Daten, die der User selbst sichern will. |

**Übernommenes Modell — „Vault = Repo":**

1. Eine **Vault** ist genau ein Verzeichnis. Es enthält ein git-Repo mit
   beliebig vielen `fotobuch/*`-Branches. Eine Vault entspricht 1:1 dem
   `repo_root` der CLI-Befehle.
2. Default-Vault-Pfad: `~/Pictures/Fotobuch` (XDG-Userdirs für `Pictures`
   via `dirs` crate, mit Fallback auf `$HOME/Fotobuch`). Wird beim
   ersten Projekt automatisch angelegt — der User wird **nicht** bei
   jedem Start nach einem Pfad gefragt.
3. **App-Settings** liegen unter dem OS-Standard-Config-Dir (`dirs::
   config_dir()` →  Linux `~/.config/fotobuch/settings.toml`,
   macOS `~/Library/Application Support/fotobuch/`, Windows
   `%APPDATA%\fotobuch\`). Inhalt: `recent_vaults: Vec<PathBuf>`,
   `last_vault: Option<PathBuf>`. Bewusst **nicht** unter
   `~/.fotobuch/` — das verstößt gegen XDG.
4. **Override-Pfade für Power-User** (Priorität von oben nach unten):
   1. CLI-Argument `fotobuch-gui --vault <path>`.
   2. Env `FOTOBUCH_VAULT=<path>`.
   3. `current_dir()`, falls darin schon `fotobuch/*`-Branches existieren
      — bewahrt das heutige „aus dem Repo heraus starten"-Verhalten.
   4. `last_vault` aus den App-Settings.
   5. Default-Vault `~/Pictures/Fotobuch` (anlegen, falls fehlt).

So bleibt der Doppelklick-Pfad reibungsfrei und der Terminal-Power-User-
Pfad genauso funktional wie heute. Eine Vault verlässt der User
explizit über „Switch Vault …" oder den entsprechenden CLI-Flag — kein
Auto-Discovery von fremden Repos.

---

## 7.1 App-Settings + Vault-Resolver

Neue Lib-Module (kommen in **Lib**, nicht GUI, weil eine spätere TUI
oder ein Daemon dieselbe Vault-Resolution braucht):

- `src/app_settings.rs` (+ Submodul-Verzeichnis falls nötig):
  - `AppSettings { recent_vaults, last_vault }`, `load() / save()`.
  - TOML, atomar geschrieben. Schema-Versionsnummer von Anfang an.
- `src/vault.rs`:
  - `resolve_vault(args, env, cwd) -> VaultPath` mit der Prio-Kette aus
    7.0. Pure Funktion, separat testbar (alle drei Inputs als Args).
  - `ensure_vault(path)`: legt Verzeichnis + leeres git-Repo an, falls
    nicht vorhanden. No-op, falls schon Vault.

Crate-Abhängigkeit: `dirs = "5"` (existiert bereits — sonst
`directories` als Drop-in).

Tests:
- `resolve_vault_priorities_cli_over_env_over_cwd_over_settings_over_default`.
- `app_settings_round_trip_preserves_recent_list_order`.
- `ensure_vault_is_idempotent_on_existing_repo`.

---

## 7.2 Projekt-Operationen in der GUI

Alle drei Lib-Calls werden über einen Worker-Roundtrip dispatcht (analog
Phase 5/6). Neue Tasks in `gui/task.rs`:

```text
ProjectNew    { config: NewConfig }
ProjectSwitch { name: String }
ProjectList                         // pull, kein Mutations-Task
```

`ProjectList` läuft synchron beim Vault-Open, weil das Ergebnis das UI
sofort braucht (Projekt-Auswahl). Implementierung: ein einmaliger
`BackgroundTask::ListProjects` direkt nach Vault-Wechsel; das Resultat
landet in `DataState::projects: Vec<ProjectInfo>`.

### 7.2.1 Toolbar / Sidebar

Links in der Toolbar: ein **Projekt-Dropdown** mit:

- der aktuellen Vault als Header (klickbar → öffnet Vault-Picker, 7.4)
- pro `fotobuch/<name>`-Branch eine Zeile, aktiver mit ✓
- Footer: `+ New project …` und `⇄ Switch vault …`

Klick auf eine Projekt-Zeile pusht `BackgroundTask::ProjectSwitch`. Vor
dem Switch prüft der Worker `git status` (Lib tut das schon, siehe
`project_switch` — wirft bei dirty WT). Bei Fehler → Toast (Phase 6.2.4).

### 7.2.2 New-Project-Dialog

Modal `new_project_dialog.rs`. Felder (englisch):

- `Name` (validiert via `validate_project_name`).
- `Width / Height (mm)` — Default 210/297 (A4 Hochformat).
- `Bleed (mm)` — Default 3.
- `Margin (mm)` — Default 10.
- Checkbox `With cover`. Wenn aktiv: Eingaben für
  `Cover width / height (mm)` (Default = `2 * width / height`) und
  Radio-Group `Spine: auto (mm/10 pages)` vs. `Spine: fixed (mm)`.

Submit pusht `BackgroundTask::ProjectNew { config }`. Das CLI-Validierungs-
gate „mit `--with-cover` muss `--spine-grow-per-10-pages-mm` ODER
`--spine-mm` gesetzt sein" wird im Dialog UI-seitig abgebildet (Submit
disabled), bleibt aber in der Lib als Defense-in-Depth (existiert dort
heute nur im CLI-Handler — Phase 7.2.2 zieht den Check in
`commands::project::new::project_new` mit, damit beide Frontends
denselben Vertrag haben).

Nach erfolgreichem `ProjectNew` ruft die Pipeline automatisch
`ProjectSwitch <name>` auf (User-Erwartung: „neu erstellt → ich bin
drin").

### 7.2.3 Commits

1. `feat(lib): app_settings + vault resolver`
2. `refactor(lib): move with-cover/spine validation into project_new`
3. `feat(gui): project switcher dropdown in toolbar`
4. `feat(gui): new project dialog`

---

## 7.3 History-Panel

Read-only, kein eigener Worker-Task pro Frame. Beim Öffnen des Panels
einmal `BackgroundTask::History { count: 100 }` pushen, Ergebnis in
`DataState::history: Vec<HistoryEntry>` cachen, bei jedem
`CommandDone` mit `new_state` invalidieren (neuer Commit liegt vor).

UI: rechtes Slide-Out-Panel oder eigenes Dialog-Fenster (entscheidet
sich am Layout — vorerst einfaches `egui::Window::new("History")`).
Eintrag = `Datum   Message`. **Keine** Undo/Redo-Buttons im Panel —
Undo/Redo ist bereits über Toolbar/Hotkeys verfügbar; das History-
Panel ist Read-only-Information.

Commit: `feat(gui): history panel reading commit log`.

---

## 7.4 First-Run + Vault-Wechsel

### 7.4.1 First-Run-Erkennung

`AppSettings::load()` liefert `last_vault: None` und `recent_vaults: []`
→ First-Run. Statt direkt das Hauptfenster zu zeigen, blende die
**Welcome-Modal** ein:

```
fotobuch
─────────────────────────────────
  ○ Create your first photobook   →  öffnet New-Project-Dialog
  ○ Open an existing folder        →  Folder-Picker (rfd)
  ─────────────────────────────────
  Advanced ▾
    Vault location: [~/Pictures/Fotobuch]  [Browse …]
```

Der Default-Pfad ist vorbelegt und bleibt als Standard versteckt — ein
nicht-technischer User klickt einfach „Create your first photobook"
und sieht das Verzeichnis nie. Der Power-User klappt „Advanced" auf
und ändert den Pfad.

Nach Auswahl wird die Vault per `ensure_vault()` materialisiert und
in `last_vault + recent_vaults` aufgenommen.

### 7.4.2 Vault-Wechsel im laufenden Betrieb

Toolbar → `⇄ Switch vault …` öffnet einen Folder-Picker (rfd). Der
gewählte Pfad wird via `ensure_vault()` validiert (oder bei leerem
Verzeichnis als neue Vault initialisiert — Confirm-Popup). Die GUI
schließt den aktuellen Project-State und re-init mit dem neuen
`repo_root`. State-Reset analog zu `FotobuchApp::new`.

### 7.4.3 Recent-Vaults-Untermenü

Im Switch-Dialog optional eine Liste der letzten 5 Vaults aus
`AppSettings.recent_vaults` als Quick-Pick.

---

## 7.5 User-Szenarien (Sanity-Check)

**Anna — Hobby-Fotografin, kein Git, kein Terminal.**
Doppelklick aufs Desktop-Icon → Welcome-Modal → klickt „Create your
first photobook" → gibt „Sommer-Italien" + A4 ein → Klick OK →
landet im leeren Layout, sieht den Pool. Drag-Drop von Foto-Ordner
auf Fenster (Phase 6.0.2) startet Add. Sie wird nie mit dem Begriff
„Vault" oder „Repo" konfrontiert; das Foto-Buch ist einfach „da".

**Beatrice — Mehr-Bücher-Hobbyistin.**
Hat letztes Jahr „Sommer-Italien" angelegt. Startet die App,
sieht direkt die Layout-Ansicht des zuletzt geöffneten Projekts.
Klickt links im Toolbar-Dropdown → sieht beide Projekte → wechselt
zu „Sommer-Italien" → fügt neue Fotos hinzu.

**Peter — Power-User, CLI + Git, mehrere Klienten-Repos.**
Hat `~/work/clients/foo/photobooks` (Repo) und
`~/work/clients/bar/photobooks` (Repo). Startet die GUI per
`fotobuch-gui --vault ~/work/clients/foo/photobooks`. Macht
Inkremental-Edits und committed wie gewohnt. Wechselt das Repo
über `--vault` (zweite Instanz parallel) oder über
`Switch vault …` in der laufenden GUI.

**Rita — kommt nach 6 Monaten zurück.**
Findet ihr Buch sofort wieder, ohne sich an „wo war das
nochmal?" erinnern zu müssen — `last_vault` lädt automatisch.

**Falls die letzte Vault gelöscht/verschoben wurde:**
`ensure_vault()` schlägt fehl, der Resolver fällt durch zur
nächst-niedrigeren Prio (Default-Vault). Ein nicht-blockierender
Toast erklärt den Fallback. Recent-Vaults-Eintrag wird gepurged.

---

## 7.6 Akzeptanzkriterien

- [ ] Erststart ohne Argumente und ohne `~/.config/fotobuch/`
      zeigt die Welcome-Modal.
- [ ] „Create your first photobook" legt Default-Vault an, erstellt
      Projekt, schaltet auf den neuen Branch und zeigt das leere
      Layout.
- [ ] Folgende Starts laden `last_vault` automatisch ohne Modal.
- [ ] Toolbar zeigt aktuelles Projekt + Liste aller `fotobuch/*`-
      Branches; Klick wechselt via `project_switch`.
- [ ] „New Project" öffnet Dialog, Submit ruft `project_new` und
      switcht direkt dorthin.
- [ ] „Switch vault …" öffnet Folder-Picker, ein leeres
      Verzeichnis kann zur neuen Vault initialisiert werden
      (Confirm).
- [ ] History-Panel listet die letzten 100 Commits des aktuellen
      Branches.
- [ ] CLI-Flag `--vault` und `FOTOBUCH_VAULT` überschreiben
      `last_vault`.
- [ ] Switch bei dirty Working-Tree zeigt Error-Toast (Lib lehnt ab).
- [ ] Default-Vault lebt in `~/Pictures/Fotobuch`, App-Settings in
      `dirs::config_dir()/fotobuch/` — **nicht** in `~/.fotobuch/`.

## 7.7 Was NICHT in Phase 7 gehört

- Cloud-Sync / Multi-Device.
- Vault-Backup-Wizard.
- Projekt-Klonen (Branch kopieren als neues Projekt).
- Custom Templates pro Projekt jenseits der heutigen `<name>.typ`-
  Datei.
- Rename-Project (git-Branch umbenennen) — separates Feature.
- Per-Project-Settings im App-Settings-File. Project-spezifische
  Config bleibt in `<name>.yaml`.
