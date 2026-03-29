# `fotobuch add`

## CLI-Interface

```text
$ fotobuch add --help
Add photos to the project

Usage: fotobuch add [OPTIONS] <PATHS>...

Arguments:
  <PATHS>...  Directories or individual files to add

Options:
      --allow-duplicates  Allow adding files with identical content
  -h, --help              Print help
```

## Verhalten

Scannt Verzeichnisse nach Bilddateien, gruppiert sie, liest EXIF-Metadaten, erkennt Duplikate und fügt sie zum Projekt hinzu.

### Gruppierungslogik

Jedes Verzeichnis das **direkt** Bilddateien enthält wird eine Gruppe.

```text
~/Fotos/Urlaub/
├── Tag1/
│   ├── IMG_001.jpg  ← Tag1-Gruppe
│   └── IMG_002.jpg
├── Tag2/
│   └── IMG_003.jpg  ← Tag2-Gruppe
└── panorama.jpg     ← Urlaub-Gruppe (root)
```

**Gruppenname**: Relativer Pfad ab dem `add`-Argument. Bei Einzeldateien: Elternverzeichnis.

### Zeitstempel-Heuristik (sort_key)

Pro Gruppe wird ein `sort_key` (ISO 8601) bestimmt. Erste verfügbare Quelle gewinnt:

1. Ordnername parsen: `2024-01-15_Urlaub` → `2024-01-15T00:00:00`
2. Frühestes EXIF-Datum (`DateTimeOriginal`) aller Fotos der Gruppe
3. Früheste Datei-mtime

### Duplikaterkennung

Methode: Partieller Hash (erste 64 KB + letzte 64 KB + Dateigröße) via Blake3.

| Situation                             | Aktion                                              |
| ------------------------------------- | --------------------------------------------------- |
| Selber absoluter Pfad bereits im YAML | Überspringen                                        |
| Selber Hash, anderer Pfad             | Warnung + Überspringen (außer `--allow-duplicates`) |
| Gruppe existiert bereits              | Fotos zur existierenden Gruppe hinzufügen           |

### ID-Generierung

Format: `<group>/<filename_with_ext>`. Bei Namenskollision innerhalb der Gruppe: Suffix `_1`, `_2`, ... vor die Endung anhängen.

## Ablauf

1. StateManager öffnen (committed ggf. manuelle User-Edits)
2. Pfade scannen
3. Duplikate erkennen (Pfad-Check + Hash-Check gegen existierende Fotos)
4. Gruppen mergen: existierende Gruppen erweitern, neue hinzufügen
5. Gruppen nach sort_key sortieren
6. YAML schreiben + Git commit: `add: N photos in M groups`
