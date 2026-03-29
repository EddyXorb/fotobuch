# `fotobuch config`

## CLI-Interface

```text
$ fotobuch config
```

Rein lesend — verändert nichts. (`StateManager::open()` committed jedoch ausstehende Nutzer-Edits bevor die Config gelesen wird.)

## Verhalten

Zeigt die vollständig aufgelöste Konfiguration als kommentiertes YAML. Explizit gesetzte Werte erscheinen unmarkiert, Defaultwerte erhalten den Kommentar `# default`.

## Ausgabe-Format

```yaml
config:
  book:
    title: Mein Fotobuch
    page_width_mm: 420.0
    page_height_mm: 297.0
    bleed_mm: 3.0
    margin_mm: 10.0              # default
    gap_mm: 5.0                  # default
  page_layout_solver:
    seed: 42                     # default
    population_size: 200         # default
    max_generations: 1000        # default
  preview:
    show_filenames: false        # default
    max_preview_px: 800          # default
  book_layout_solver:
    page_target: 20              # default
```

Die Ausgabe ist gültiges YAML — Werte können direkt in `<name>.yaml` kopiert werden, um Defaults zu überschreiben.

## Design-Entscheidung: Default-Erkennung

`serde_yaml` kennt keine eingebaute "explizit gesetzt vs. Default"-Unterscheidung. Die Lösung: zwei Deserialisierungen — einmal als `serde_yaml::Value` (enthält nur explizit gesetzte Keys) und einmal als `ProjectConfig` (enthält alle Felder mit angewandten Defaults). Durch Vergleich der Key-Mengen lässt sich jeder Wert annotieren.

## Zusammenspiel mit `project new`

`project new` schreibt eine vollständige YAML mit allen Feldern. `config` ist v.a. nützlich, wenn der Benutzer Teile der YAML gelöscht hat oder wissen möchte, welche Einstellungen verfügbar sind.
