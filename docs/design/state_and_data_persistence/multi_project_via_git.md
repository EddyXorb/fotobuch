# Multi-Projekt-Verwaltung via Git-Branches

## Idee

Jedes Fotobuch-Projekt lebt auf einem eigenen Git-Branch `fotobuch/<projektname>`. Pro Projekt gibt es eine eigene YAML-Datei `<projektname>.yaml`. Der aktive Branch bestimmt, welches Projekt bearbeitet wird — kein `--project`-Flag nötig.

## Projektnamen

Erlaubt: `[a-zA-Z][a-zA-Z0-9._-]*`, maximal 50 Zeichen. Verboten: `..`, rein numerisch, reservierte Namen.

## Verzeichnisstruktur

```
mein-fotobuch/
├── .git/
├── .gitignore                     # .fotobuch/ + *.pdf + final.typ
├── .fotobuch/cache/
│   ├── urlaub/preview/
│   ├── urlaub/final/
│   └── hochzeit/preview/
├── urlaub.yaml                    # Branch fotobuch/urlaub
├── urlaub.typ
├── hochzeit.yaml                  # Branch fotobuch/hochzeit
└── hochzeit.typ
```

Jeder Branch trackt nur **seine eigenen** Dateien (`{name}.yaml`, `{name}.typ`).

## Lifecycle

### Neues Projekt: `fotobuch project new <name>`

1. Branch `fotobuch/<name>` erstellen
2. `<name>.yaml` mit Default-Config anlegen
3. `<name>.typ` — Typst-Template anlegen
4. Cache-Verzeichnisse anlegen
5. Nur eigene Dateien committen

Bei einem weiteren Projekt im selben Repo: Dateien des vorherigen Projekts aus dem Index entfernen (`git rm --cached`), neue hinzufügen.

### Projekt wechseln: `fotobuch project switch <name>`

1. Uncommitted Changes am aktuellen YAML prüfen → ggf. auto-commit
2. `git switch fotobuch/<name>`

### Projekte auflisten: `fotobuch project list`

Alle Branches mit Prefix `fotobuch/` auflisten, aktuellen markieren.

## Projekt-Erkennung im StateManager

```
StateManager::open(project_root)
├─ Aktuellen Branch-Namen lesen
├─ Prefix "fotobuch/" prüfen → Fehler wenn nicht
├─ Projektname = Branch-Name ohne Prefix
├─ YAML-Pfad = project_root / "{name}.yaml"
└─ Cache-Pfad = project_root / ".fotobuch/cache/{name}/"
```

## Typst-Templates

Pro Projekt gibt es **ein** Template `{name}.typ`. Bei `build --release` wird eine nicht-getrackte Kopie `final.typ` erzeugt (`is_final = true`).
