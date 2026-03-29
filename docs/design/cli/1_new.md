# `fotobuch project new`

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

Verwandte Subkommandos:

```text
fotobuch project switch <name>   # Wechsel zu einem anderen Projekt
fotobuch project list            # Zeigt alle vorhandenen Projekte
```

## Verhalten

Mehrere Projekte koexistieren im selben Git-Repository auf separaten Branches (`fotobuch/<name>`). Jedes Projekt hat eine eigene YAML-Datei (`<name>.yaml`) und ein eigenes Typst-Template (`<name>.typ`). Das finale Template (`final.typ`) wird erst bei `build --release` generiert und ist nicht getrackt.

### Erstes Projekt (kein `fotobuch/*`-Branch existiert)

Erstellt ein neues Verzeichnis mit dem Namen des Projekts. Der Ordnername kann später umbenannt werden — die YAML- und Typst-Dateien jedoch nicht. Beim ersten Aufruf gibt das Kommando eine kurze Einführung aus (max. 10 Zeilen): Überblick über den Workflow, Hinweis dass `.yaml` und `.typ` manuell angepasst werden dürfen, und dass alle Änderungen versioniert sind und rückgängig gemacht werden können.

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

`.gitignore` enthält `.fotobuch/`, `*.pdf` und `final.typ`.

### Weiteres Projekt (bereits auf einem `fotobuch/*`-Branch)

Erstellt `<name>.yaml` und `<name>.typ` im Repository-Root, legt Cache-Verzeichnisse an und wechselt auf den neuen Branch `fotobuch/<name>`. Das alte Projekt wird aus dem Git-Index entfernt (Dateien bleiben auf der Platte). Jeder Branch `fotobuch/<name>` trackt ausschließlich die Dateien des zugehörigen Projekts.

## Typst-Template

Das Template wird aus einer eingebetteten Vorlage erzeugt. Der Schalter `#let is_final = false` steuert das Verhalten:

- `false` (Preview-Modus): Wasserzeichen, Annotationen, Seitenzahlen gemäß Config
- `true` (wird in `final.typ` auf `true` gesetzt): kein Wasserzeichen, druckfertig

`final.typ` wird bei `build --release` generiert und ist gitignored.

## Namensvalidierung

- Muss mit `[a-zA-Z]` beginnen
- Darf nur `[a-zA-Z0-9._-]` enthalten
- Maximallänge: 50 Zeichen
- Darf nicht `..` enthalten (Pfadtraversal)
- Darf nicht `fotobuch` sein (reserviert als Branch-Präfix)

## Commit-Message

`new: <name>, <W>x<H>mm, <B>mm bleed`
