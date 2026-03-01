# photobook-solver

Verteilt Fotos aus Ordnern mit Zeitstempel-Namen auf Seiten und exportiert ein PDF via Typst.

## Voraussetzungen

- Rust (stable) – https://rustup.rs
- Systemfonts (für Typst)

## Bauen

```bash
cargo build --release
```

## Verwendung

```bash
./target/release/photobook-solver \
  --input /pfad/zu/fotoordnern \
  --output fotobuch.pdf
```

### Optionen

| Flag | Standard | Beschreibung |
|------|----------|--------------|
| `--input` | – | Wurzelverzeichnis mit Unterordnern |
| `--output` | `photobook.pdf` | Ausgabe-PDF |
| `--write-typ` | `true` | `.typ`-Datei neben PDF schreiben |
| `--page-width` | `297.0` | Seitenbreite in mm (A4 quer) |
| `--page-height` | `210.0` | Seitenhöhe in mm |
| `--margin` | `10.0` | Rand in mm |
| `--gap` | `3.0` | Abstand zwischen Fotos in mm |
| `--max-photos` | `4` | Maximale Fotos pro Seite |

## Ordner-Namensformate

Folgende Zeitstempel-Formate werden erkannt:

- `2024-07-15_Urlaub_Italien`
- `2024-07-15`
- `20240715_Ferien`
- `2024-07-15_18-30-00`

Ordner ohne erkennbaren Zeitstempel werden ans Ende sortiert.

## Ausgabe

- `fotobuch.pdf` – finales PDF (direkt bei Saal Digital hochladbar)
- `fotobuch.typ` – Typst-Quelldatei für manuelle Nachbearbeitung

### Manuelle Anpassung der .typ-Datei

Jedes Bild ist als `#place(...)` mit exakten mm-Koordinaten abgelegt:

```typst
#place(top + left, dx: 10.00mm, dy: 10.00mm,
  block(width: 135.50mm, height: 190.00mm, clip: true,
    image("/pfad/zum/foto.jpg", width: 135.50mm, height: 190.00mm, fit: "cover")))
```

Nach Anpassung neu kompilieren:
```bash
typst compile fotobuch.typ fotobuch.pdf
```

## Projektstruktur

```
src/
  main.rs          – CLI-Einstiegspunkt
  models.rs        – Datenstrukturen (Photo, Page, Placement, ...)
  scanner.rs       – Ordner einlesen, Zeitstempel parsen, EXIF auslesen
  solver.rs        – Bildverteilung auf Seiten + Layout-Algorithmus
  typst_export.rs  – .typ-Generierung + Typst-Kompilierung zu PDF
```

## Tests

```bash
cargo test
```
